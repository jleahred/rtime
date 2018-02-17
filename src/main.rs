extern crate termion;


use std::process::{Child, Command, Stdio};
use std::time::Instant;
use std::thread;
use std::sync::mpsc::{sync_channel, Receiver, SyncSender};

use termion::input::MouseTerminal;
use termion::raw::IntoRawMode;

use std::os::unix::net::{UnixListener, UnixStream};

enum Print {
    Line(String),
    ElapsedTime,
}


struct Status {
    start: Instant,
}

impl Status {
    fn init() -> Status {
        Status {
            start: Instant::now(),
        }
    }
}

fn main() {
    use std::os::unix::io::FromRawFd;
    use std::os::unix::io::IntoRawFd;

    if std::env::args().count() == 1 {
        println!("missing command to execute");
        println!("\nussage  rtime <command>\n\n");
        return;
    }

    let (send_print, recv_print) = sync_channel(1);
    let (send_finished, recv_finished) = sync_channel(1);

    let (unix_listener, stream) = create_unix_stream();


    //  create command to execute
    let comm_vec: std::vec::Vec<_> = std::env::args().skip(1).collect();
    let child_proc = Command::new("sh")
        .arg("-c")
        .arg(format!("{0}", comm_vec.join(" ")))
        .stdout(unsafe { Stdio::from_raw_fd(stream.try_clone().unwrap().into_raw_fd()) })
        .stderr(unsafe { Stdio::from_raw_fd(stream.try_clone().unwrap().into_raw_fd()) })
        //.stdout(Stdio::piped())
        //.stderr(Stdio::piped())
        .spawn()
        .expect("Problem running the command");

    thread_notif_end(child_proc, send_finished.clone(), stream);
    thread_read_socket(unix_listener, send_print.clone());
    thread_send_print_elapsed_time(send_print.clone(), recv_finished);
    drop(send_print);

    let process_print = |status, print| match print {
        Print::ElapsedTime => print_elapsed_time(status),
        Print::Line(line) => print_line(&line, status),
    };

    recv_print.iter().fold(Status::init(), |status, received| {
        process_print(status, received)
    });

    println!("\n");
}


fn thread_notif_end(mut child_proc: Child, send_finished: SyncSender<()>, mut stream: UnixStream) {
    thread::spawn(move || {
        use std::io::prelude::*;
        let _ = child_proc.wait();
        let _ = send_finished.send(());
        let _ = stream.write_all(b"__RTIME_END__");
    });
}


fn create_unix_stream() -> (UnixListener, UnixStream) {
    use std::fs;

    use std::path::Path;

    let socket_path = Path::new("/tmp/rtime");
    if socket_path.exists() {
        fs::remove_file(&socket_path).unwrap();
    }
    let ul = UnixListener::bind(&socket_path).expect("failed to bind socket");
    let us = UnixStream::connect(&socket_path).unwrap();
    (ul, us)
}


fn thread_read_socket(unix_listener: UnixListener, send_print: SyncSender<Print>) {
    use std::io::BufRead;

    thread::spawn(move || {
        match unix_listener.accept() {
            Ok((socket, _)) => {
                let child_buf = std::io::BufReader::new(socket);
                for line in child_buf.lines() {
                    let line_s = line.unwrap_or("".to_owned());
                    if line_s == "__RTIME_END__".to_owned() {
                        break;
                    }
                    let _ = send_print.clone().send(Print::Line(line_s));
                    let _ = send_print.clone().send(Print::ElapsedTime);
                }
            }
            Err(_) => { /* connection failed */ }
        }
    });
}



fn thread_send_print_elapsed_time(sender: SyncSender<Print>, recv_finished: Receiver<()>) {
    use std::time::Duration;

    thread::spawn(move || {
        loop {
            match recv_finished.recv_timeout(Duration::from_millis(250)) {
                Ok(_) => break,
                Err(_) => {
                    let _ = sender.send(Print::ElapsedTime);
                }
            }
        }
    });
}

fn print_elapsed_time(status: Status) -> Status {
    use std::io::{self, Write};

    let iostdout = io::stdout()
        .into_raw_mode()
        .and_then(|iostdout| Ok(MouseTerminal::from(iostdout)));

    match iostdout {
        Ok(mut stdout) => {
            let _ = write!(
                stdout,
                "\r{}{} ...",
                termion::clear::CurrentLine,
                get_string_time(status.start.elapsed().as_secs()),
            );
            let _ = stdout.flush();
        }
        _ => (),
    }

    status
}

fn print_line(line: &str, status: Status) -> Status {
    fn write_line(l: &str, string_time: &str) {
        use std::io::{self, Write};

        let iostdout = io::stdout()
            .into_raw_mode()
            .and_then(|iostdout| Ok(MouseTerminal::from(iostdout)));

        match iostdout {
            Ok(mut stdout) => {
                let _ = write!(
                    stdout,
                    "\r{}{}|{}\n",
                    termion::clear::CurrentLine,
                    string_time,
                    l
                );
                let _ = stdout.flush();
            }
            _ => (),
        }
    };

    match termion::terminal_size() {
        Ok((width, _)) => {
            let string_time = get_string_time(status.start.elapsed().as_secs());
            let spaces = " ".repeat(string_time.len());

            let vlines = split_str_len(line, width as usize - string_time.len() - 1);
            if vlines.len() > 0 {
                write_line(&vlines[0], &string_time);
            }
            vlines.iter().skip(1).fold((), |(), l| {
                write_line(l, &spaces);
            });
        }
        _ => (),
    }

    status
}


fn get_string_time(total_seconds: u64) -> String {
    let div_rem = |dividend, divisor| (dividend / divisor, dividend % divisor);
    let (total_minuts, seconds) = div_rem(total_seconds, 60);
    let (total_hours, minuts) = div_rem(total_minuts, 60);
    match (total_hours > 0, total_minuts > 0) {
        (true, _) => format!("{}h{:02}m{:02}s", total_hours, minuts, seconds),
        (false, true) => format!("{}m{:02}s", minuts, seconds),
        (false, false) => format!("{}s", seconds),
    }
}

fn split_str_len(s: &str, len: usize) -> Vec<String> {
    fn split_(s: &str, len: usize, mut v: Vec<String>) -> (String, Vec<String>) {
        let split_on = match len < s.len() {
            true => len,
            false => s.len(),
        };
        match s.len() {
            0 => (s.to_owned(), v),
            _ => {
                let (l, r) = s.split_at(split_on);
                v.push(l.to_owned());
                split_(r, len, v)
            }
        }
    }
    let (_, v) = split_(s, len, vec![]);
    v
}

#[test]
fn test_split_str_len() {
    assert_eq!(
        split_str_len("123456789012", 4),
        vec!["1234", "5678", "9012"]
    );
    assert_eq!(split_str_len("1234567890", 4), vec!["1234", "5678", "90"]);
}
