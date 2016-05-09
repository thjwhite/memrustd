extern crate mio;
extern crate slab;

const SERVER: mio::Token = mio::Token(0);

struct Connection {
    socket: mio::tcp::TcpStream,
    token: mio::Token,
    interest: mio::EventSet,

    // output buffer
    out_buf: std::io::Cursor<Vec<u8>>,

    // input buffer
    in_buf: std::io::Cursor<Vec<u8>>
}

impl Connection {
    
    fn new(socket: TcpStream, token: Token) -> Connection {
        Connection {
            socket: socket,
            token: token,
            interest: mio::EventSet::hup(),
            out_buf: Cursor::new(Vec::new()),
            in_buf: Cursor::new(Vec::new())
        }
    }

    fn handle_readable(&mut self, event_loop: &mut EventLoop<Server>) -> std::io::Result<()> {
        match self.socket.try_read_buf(&mut self.in_buf) {
            Ok(None) => {
                self.interest.inset(mio::EventSet::readable());
            }
            Ok(Some(num_bytes)) => {
                self.out_buf.write_all(self.in_buf.get_ref());

                self.interest.remove(mio::EventSet::readable());
                self.interest.insert(mio::EventSet::writable());
            }
            Err(e) => {
                println!("Error reading");
                self.interest.remove(mio::EventSet::readable());
            }
        }

        event_loop.reregister(&self.socket,
                              self.token,
                              self.interest,
                              mio::PollOpt::edge() | mio::PollOpt::oneshot())
    }

    fn handle_writable(&mut self, event_loop: &mut EventLoop<Server>) -> std::io::Result<()> {
        match self.socket.try_write_buf(&mut self.out_buf) {
            Ok(None) => {
                self.interest.insert(mio::EventSEt::writable());
            }
            Ok(Some(num_bytes)) => {
                // todo may need to change this behavior later
                self.interest.remove(EventSet::writable());
                self.interest.insert(EventSet::readable());
            }
            Err(e) => {
                println!("Error writing");
            }
        }

        event_loop.reregister(&self.socket,
                              self.token,
                              self.interest,
                              mio::PollOpt::edge() | mio::PollOpt::oneshot())
    }

}

struct Server {
    server: mio::tcp::TcpListener,
    connections: slab::Slab<mio::Connection, mio::Token>
}

impl Server {

    fn new(server: mio::tcp::TcpListener) -> Pong {
        let slab = mio::util::Slab::new_starting_at(mio::Token(1), 1024);

        Pong {
            server: server,
            connections: slab
        }
    }

}

impl mio::Handler for Server {
    type Timeout = ();
    type Message = ();

    fn ready(&mut self,
             event_loop: &mut mio::EventLoop<Pong>,
             token: mio::Token,
             events: mio::EventSet) {
        match token {
            SERVER =>  {
                if !events.is_readable() {
                    println!("server event, not readable");
                }
                println!("the server socket is ready to accept a connection");
                match self.server.accept() {
                    Ok(Some((socket, _))) => {
                        // ignore the SocketAddr for now (the _)
                        // all we care about is the TcpListener
                        match self.connections.insert_with(|token| 
                                                           Connection::new(socket, token)) {
                            Some(_) => {
                                // successfully inserted into our connection slab
                                match event_loop.register(&self.connections[token].socket,
                                                          token,
                                                          mio::EventSet::readable(),
                                                          mio::PollOpt::edge() |
                                                          mio::PollOpt::oneshot()) {
                                    Ok(_) => {}, // success
                                    Err(e) => {
                                        println!("Failed to register Connection");
                                        self.connections.remove(token);
                                    }
                                }
                            }
                            None => {
                                println!("Failed to insert connection into slab");
                            }
                        }
                    }
                    Ok(None) => {
                        println!("the server socket wasn't actually ready");
                    }
                    Err(e) => {
                        println!("listener.accept() errored: {}", e);
                        event_loop.shutdown();
                    }
                }
            }
            _ => {
                // assume all other tokens are existing client connections.
                if events.is_readable() {
                    self.connections[token].handle_readable(event_loop);
                }

                if events.is_writable() {                
                    self.connections[token].handle_writable(event_loop);
                }
                
                if self.connections[token].is_closed() {
                    self.connections.remove(token);
                }
            }
        }
    }
}

fn main() {
    let address = "0.0.0.0:6567".parse().unwrap();
    let server = mio::tcp::TcpListener::bind(&address).unwrap();

    let mut event_loop = mio::EventLoop::new().unwrap();
    event_loop.register(&server, SERVER, mio::EventSet::readable(), mio::PollOpt::level());

    println!("running pingpong server");
    event_loop.run(&mut Server { server: server });
}
