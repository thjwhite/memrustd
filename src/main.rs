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

    fn set_kill_on_tick(&mut self) {
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

    fn handle_accept(&mut self,
                     socket: mio::tcp::TcpStream,
                     event_loop: &mut mio::EventLoop<Server>) -> std::io::Result<()> {
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
                    Ok(_) => {
                        //success
                        Ok(())
                    },
                    Err(e) => {
                        println!("Failed to register connection in the event loop");
                        self.connections.remove(new_token);
                        Err(e)
                    }
                }
            }
            None => {
                println!("Failed to insert connection into slab");
                Err(std::io::Error::new(std::io::ErrorKind::Other,
                                        "failed to insert connection into slab"))
            }
        }
    }

    fn loop_accept(&mut self,
                   event_loop: &mut mio::EventLoop<Server>) -> std::io::Result<()> {
        loop {
            match self.server.accept() {
                Ok(Some((socket, _))) => {
                    match self.handle_accept(socket, event_loop) {
                        Ok(_) => {}
                        Err(e) => {
                            return Err(e);
                        }
                    }
                }
                Ok(None) => {
                    println!("No more pending connection requests to accept");
                    return Ok(());
                }
                Err(e) => {
                    println!("listener.accept() errored: {}", e);
                    return Err(e);
                }
            }
        }
    }

}

impl mio::Handler for Server {
    type Timeout = ();
    type Message = ();

    fn tick(&mut self, event_loop: &mut mio::EventLoop<Server>) {
        let mut kill_tokens = Vec::new();
        for conn in self.connections.iter_mut() {
            if conn.kill_on_tick {
                kill_tokens.push(conn.token);
            }
        }

        for token in kill_tokens {
            match self.connections.remove(token) {
                Some(conn) => {
                    println!("kill connection {}", token.as_usize());
                    event_loop.deregister(&conn.socket);
                }
                None => {
                    println!("unable to kill connection {}", token.as_usize());
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
                match self.loop_accept(event_loop) {
                    Ok(_) => {}
                    Err(e) => {
                        println!("Failure in loop_accept: {}", e);
                    }
                }
            }
            _ => {
                if events.is_error() {
                    println!("Error event");
                    self.connections[event_token].set_kill_on_tick();
                    return;
                }

                if events.is_hup() {
                    println!("HUP event");
                    self.connections[event_token].set_kill_on_tick();
                    return;
                }

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
