extern crate mio;
extern crate slab;
extern crate bytes;

const SERVER: mio::Token = mio::Token(0);

struct Connection {
    socket: mio::tcp::TcpStream,
    token: mio::Token,
    interest: mio::EventSet,

    // output buffer, so we can send stuff
    out_buf: Option<bytes::MutByteBuf>,

    // input buffer
    in_buf: Option<bytes::ROByteBuf>
}

impl Connection {
    
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
                                    Ok(_) => {},
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
                self.connections[token].ready(event_loop, events);
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
