use std::future::Future;
use std::net::SocketAddr;

use crate::server::configuration::ServerConfiguration;
use crate::server::server_handle::ServerHandle;

use super::IncomingStream;

/// An HTTP server to handle incoming connections for Pavex applications.  
/// It handles both HTTP1 and HTTP2 connections.
///
/// # Example
///
/// ```rust
/// use std::net::SocketAddr;
/// use pavex::server::Server;
///
/// # #[derive(Clone)] struct ApplicationState;
/// # async fn router(_req: hyper::Request<hyper::body::Incoming>, _state: ApplicationState) -> pavex::response::Response { todo!() }
/// # async fn t() -> std::io::Result<()> {
/// # let application_state = ApplicationState;
/// let addr = SocketAddr::from(([127, 0, 0, 1], 8080));
///
/// Server::new()
///     .bind(addr)
///     .await?
///     // Both the routing function and the application state will usually
///     // be code-generated by Pavex, starting from your `Blueprint`.
///     // You don't have to define them manually!
///     .serve(router, application_state)
///     // The `serve` method returns a `ServerHandle` that you can use to
///     // interact with the server.
///     // Calling `.await` on the handle lets you wait until the server
///     // shuts down.
///     .await;
/// # Ok(())
/// # }
/// ```
///
/// # Configuration
///
/// [`Server::new`] returns a new [`Server`] with default configuration.  
/// You can customize the server default settings by creating your own [`ServerConfiguration`]
/// and invoking [`Server::set_config`].
///
/// # Architecture
///
/// By default, [`Server::serve`] creates a worker per CPU core and distributes connection from an
/// acceptor thread using a round-robin strategy.
///
/// Each worker has its own single-threaded [`tokio`] runtime—there is no work stealing across
/// workers.  
/// Each worker takes care to invoke your routing and request handling logic, with the help
/// of [`hyper`].
#[must_use = "You must call `serve` on a `Server` to start listening for incoming connections"]
pub struct Server {
    config: ServerConfiguration,
    incoming: Vec<IncomingStream>,
}

impl Default for Server {
    fn default() -> Self {
        Self::new()
    }
}

impl Server {
    /// Create a new [`Server`] with default configuration.
    pub fn new() -> Self {
        Self {
            config: ServerConfiguration::default(),
            incoming: Vec::new(),
        }
    }

    /// Configure this [`Server`] according to the values set in the [`ServerConfiguration`]
    /// passed as input parameter.
    /// It will overwrite any previous configuration set on this [`Server`].
    ///
    /// If you want to retrieve the current configuration, use [`Server::get_config`].
    pub fn set_config(mut self, config: ServerConfiguration) -> Self {
        self.config = config;
        self
    }

    /// Get a reference to the [`ServerConfiguration`] for this [`Server`].
    ///
    /// If you want to overwrite the existing configuration, use [`Server::set_config`].
    pub fn get_config(&self) -> &ServerConfiguration {
        &self.config
    }

    /// Bind the server to the given address: the server will accept incoming connections from this
    /// address when started.  
    /// Binding an address may fail (e.g. if the address is already in use), therefore this method
    /// may return an error.  
    ///
    /// # Related
    ///
    /// Check out [`Server::listen`] for an alternative binding mechanism as well as a
    /// discussion of the pros and cons of [`Server::bind`] vs [`Server::listen`].
    ///
    /// # Note
    ///
    /// A [`Server`] can be bound to multiple addresses: just call this method multiple times with
    /// all the addresses you want to bind to.
    ///
    /// # Example: bind one address
    ///
    /// ```rust
    /// use std::net::SocketAddr;
    /// use pavex::server::Server;
    ///
    /// # async fn t() -> std::io::Result<()> {
    /// let addr = SocketAddr::from(([127, 0, 0, 1], 8080));
    ///
    /// Server::new()
    ///     .bind(addr)
    ///     .await?
    ///     # ;
    ///     // [...]
    /// # Ok(())
    /// # }
    /// ```
    ///
    /// # Example: bind multiple addresses
    ///
    /// ```rust
    /// use std::net::SocketAddr;
    /// use pavex::server::Server;
    ///
    /// # async fn t() -> std::io::Result<()> {
    /// let addr1 = SocketAddr::from(([127, 0, 0, 1], 8080));
    /// let addr2 = SocketAddr::from(([127, 0, 0, 1], 4000));
    ///
    /// Server::new()
    ///     .bind(addr1)
    ///     .await?
    ///     .bind(addr2)
    ///     .await?
    ///     # ;
    ///     // [...]
    /// # Ok(())
    /// # }
    /// ````
    pub async fn bind(mut self, addr: SocketAddr) -> std::io::Result<Self> {
        let incoming = IncomingStream::bind(addr).await?;
        self.incoming.push(incoming);
        Ok(self)
    }

    /// Ask the server to process incoming connections from the provided [`IncomingStream`].  
    ///
    /// # [`Server::listen`] vs [`Server::bind`]
    ///
    /// [`Server::bind`] only requires you to specify the address you want to listen at. The
    /// socket configuration is handled by the [`Server`], with a set of reasonable default
    /// parameters. You have no access to the [`IncomingStream`] that gets bound to the address
    /// you specified.
    ///
    /// [`Server::listen`], instead, expects an [`IncomingStream`].  
    /// You are free to configure the socket as you see please and the [`Server`] will just
    /// poll it for incoming connections.  
    /// It also allows you to interact with the bound [`IncomingStream`] directly
    ///
    /// # Example: bind to a random port
    ///
    /// ```rust
    /// use std::net::SocketAddr;
    /// use pavex::server::{IncomingStream, Server};
    ///
    /// # async fn t() -> std::io::Result<()> {
    /// // `0` is a special port: it tells the OS to assign us
    /// // a random **unused** port
    /// let addr = SocketAddr::from(([127, 0, 0, 1], 0));
    /// let incoming = IncomingStream::bind(addr).await?;
    /// // We can then retrieve the actual port we were assigned
    /// // by the OS.
    /// let addr = incoming.local_addr()?.to_owned();
    ///
    /// Server::new()
    ///     .listen(incoming);
    ///     # ;
    ///     // [...]
    /// # Ok(())
    /// # }
    /// ````
    ///
    /// # Example: set a custom socket backlog
    ///
    /// ```rust
    /// use std::net::SocketAddr;
    /// use socket2::Domain;
    /// use pavex::server::{IncomingStream, Server};
    ///
    /// # async fn t() -> std::io::Result<()> {
    /// // `0` is a special port: it tells the OS to assign us
    /// // a random **unused** port
    /// let addr = SocketAddr::from(([127, 0, 0, 1], 0));
    ///
    /// let socket = socket2::Socket::new(
    ///    Domain::for_address(addr),
    ///    socket2::Type::STREAM,
    ///    Some(socket2::Protocol::TCP),
    /// )
    /// .expect("Failed to create a socket");
    /// socket.set_reuse_address(true)?;
    /// socket.set_nonblocking(true)?;
    /// socket.bind(&addr.into())?;
    /// // The custom backlog!
    /// socket.listen(2048_i32)?;
    ///
    /// let listener = std::net::TcpListener::from(socket);
    /// Server::new()
    ///     .listen(listener.try_into()?)
    ///     # ;
    ///     // [...]
    /// # Ok(())
    /// # }
    /// ````
    ///
    /// # Note
    ///
    /// A [`Server`] can listen to multiple streams of incoming connections: just call this method
    /// multiple times!
    pub fn listen(mut self, incoming: IncomingStream) -> Self {
        self.incoming.push(incoming);
        self
    }

    /// Start listening for incoming connections.
    ///
    /// You must specify:
    ///
    /// - a handler function, which will be called for each incoming request;
    /// - the application state, the set of singleton components that will be available to
    ///   your handler function.
    ///
    /// Both the handler function and the application state are usually code-generated by Pavex
    /// starting from your [`Blueprint`](crate::blueprint::Blueprint).
    ///
    /// # Wait for the server to shut down
    ///
    /// `serve` returns a [`ServerHandle`].  
    /// Calling `.await` on the handle lets you wait until the server shuts down.
    ///
    /// # Panics
    ///
    /// This method will panic if the [`Server`] has no registered source of incoming connections,
    /// i.e. if you did not call [`Server::bind`] or [`Server::listen`] before calling `serve`.
    pub fn serve<HandlerFuture, ApplicationState>(
        self,
        handler: fn(http::Request<hyper::body::Incoming>, ApplicationState) -> HandlerFuture,
        application_state: ApplicationState,
    ) -> ServerHandle
    where
        HandlerFuture: Future<Output = crate::response::Response> + 'static,
        ApplicationState: Clone + Send + Sync + 'static,
    {
        if self.incoming.is_empty() {
            panic!("Cannot serve: there is no source of incoming connections. Please call `bind` or `listen` on the server before calling `serve`.");
        }
        ServerHandle::new(self.config, self.incoming, handler, application_state)
    }
}
