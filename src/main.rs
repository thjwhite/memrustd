extern crate mio;
extern crate slab;

use mio::TryRead;
use mio::TryWrite;
use std::io::Write;

const SERVER: mio::Token = mio::Token(0);

struct Connection {
    socket: mio::tcp::TcpStream,
    token: mio::Token,
    interest: mio::EventSet,

    // whether to completely deregister the connection on the tick()
    kill_on_tick: bool, 

    // output buffer
    out_buf: std::io::Cursor<Vec<u8>>,

    // input buffer
    in_buf: std::io::Cursor<Vec<u8>>
}

impl Connection {

    fn new(socket: mio::tcp::TcpStream, token: mio::Token) -> Connection {
        Connection {
            socket: socket,
            token: token,
            interest: mio::EventSet::all(),
            kill_on_tick: false,
            out_buf: std::io::Cursor::new(Vec::new()),
            in_buf: std::io::Cursor::new(Vec::with_capacity(1024))
        }
    }

    fn kill_on_tick(&mut self) {
        self.kill_on_tick = true;
    }

    fn handle_readable(&mut self,
                       event_loop: &mut mio::EventLoop<Server>) -> std::io::Result<()> {
        loop {
            match self.socket.try_read_buf(self.in_buf.get_mut()) {
                Ok(None) => {
                    // WouldBlock condition. If there was anything to read
                    // we have read it. Move on.
                    println!("WouldBlock on READ");
                    return Ok(());
                }
                Ok(Some(num_bytes)) => {
                    // We successfully read num_bytes amount of bytes.
                    // don't return, continue to next iteration to keep reading.
                    // might be queued.
                    println!("READ {} bytes", num_bytes);
                    self.out_buf.write_all(self.in_buf.get_ref());
                    self.in_buf = std::io::Cursor::new(Vec::new());
                    continue;
                }
                Err(e) => {
                    println!("Error reading");
                    return Err(e);
                }
            }
        }
    }

    fn handle_writable(&mut self,
                       event_loop: &mut mio::EventLoop<Server>) -> std::io::Result<()> {
        let buf_size = self.out_buf.get_ref().len();
        if buf_size > 0 {
            self.out_buf.set_position(0);
            while self.out_buf.position() < buf_size as u64 - 1 {
                match self.socket.try_write_buf(&mut self.out_buf) {
                    Ok(None) => {
                        println!("WouldBlock on WRITE");
                        self.out_buf = std::io::Cursor::new(Vec::new());
                        return Ok(());
                    }
                    Ok(Some(num_bytes)) => {
                        // todo may need to change this behavior later
                        println!("WROTE {}", num_bytes);
                        continue;
                    }
                    Err(e) => {
                        println!("Error writing");
                        return Err(e);
                    }
                }
            }
        }
        self.out_buf = std::io::Cursor::new(Vec::new());
        Ok(())
    }

}

struct Server {
    server: mio::tcp::TcpListener,
    connections: slab::Slab<Connection, mio::Token>
}

impl Server {

    fn new(server: mio::tcp::TcpListener) -> Server {
        let slab = mio::util::Slab::new_starting_at(mio::Token(1), 1024);

        Server {
            server: server,
            connections: slab
        }
    }

}

impl mio::Handler for Server {
    type Timeout = ();
    type Message = ();

    fn handle_accept(mio::tcp::TcpStream socket) {
        // ignore the SocketAddr for now (the _)
        // all we care about is the TcpListener
        match self.connections.insert_with(|new_token|
                                           Connection::new(socket, new_token)) {
            Some(new_token) => {
                // successfully inserted into our connection slab
                match event_loop.register(&self.connections[new_token].socket,
                                          new_token,
                                          mio::EventSet::all(),
                                          mio::PollOpt::edge()) {
                    Ok(_) => {}, // success
                    Err(e) => {
                        println!("Failed to register Connection");
                        self.connections.remove(new_token);
                    }
                }
            }
            None => {
                println!("Failed to insert connection into slab");
            }
        }
    }

    fn loop_accept() {
        loop {
            match self.server.accept() {
                Ok(Some((socket, _))) => {
                    self.handle_accept(socket);
                }
                Ok(None) => {
                    println!("No more pending connection requests to accept");
                }
                Err(e) => {
                    println!("listener.accept() errored: {}", e);
                    event_loop.shutdown();
                }
            }
        }
    }

    fn ready(&mut self,
             event_loop: &mut mio::EventLoop<Server>,
             event_token: mio::Token,
             events: mio::EventSet) {
        match event_token {
            SERVER =>  {
                if !events.is_readable() {
                    println!("server token event, not readable.");
                }
                println!("the server socket is ready to accept a connection");
                self.loop_accept();
            }
            _ => {
                // assume all other tokens are existing client connections.                
                if events.is_readable() {
                    println!("READ EVENT");
                    self.connections[event_token].handle_readable(event_loop);
                }

                if events.is_writable() {
                    println!("WRITE EVENT");
                    self.connections[event_token].handle_writable(event_loop);
                }
            }
        }
    }
}

fn main() {
    let address = "0.0.0.0:6567".parse().unwrap();
    let server = mio::tcp::TcpListener::bind(&address).unwrap();

    let mut event_loop = mio::EventLoop::new().unwrap();
    event_loop.register(&server, SERVER, mio::EventSet::readable(), mio::PollOpt::edge());

    println!("running pingpong server");
    event_loop.run(&mut Server::new(server));
}
