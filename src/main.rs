extern crate termion;

use std::io::BufRead;
use std::process::{Command, Stdio};
use std::time::Instant;
use std::time::Duration;
use std::thread;
use std::sync::mpsc::{sync_channel, Receiver, SyncSender};

use termion::input::MouseTerminal;
use termion::raw::IntoRawMode;


enum Print {
    Line(String),
    ElapsedTime,
    FinishedTasks,
}


struct Status {
    start: Instant,
    finished_tasks: u8,
    sender_finished: SyncSender<()>,
}

impl Status {
    fn init(send_finished: SyncSender<()>) -> Status {
        Status {
            start: Instant::now(),
            finished_tasks: 0,
            sender_finished: send_finished,
        }
    }
    fn check_exit(self) -> Self {
        if self.finished_tasks == 2 {
            let _ = self.sender_finished.send(());
        }
        self
    }
}

fn main() {
    println!("{:?}", termion::terminal_size());

    if std::env::args().count() == 1 {
        println!("missing command to execute");
        println!("\nussage  jtime <command>\n\n");
        return;
    }

    let comm_vec: std::vec::Vec<_> = std::env::args().skip(1).collect();
    let comm = Command::new("sh")
        .arg("-c")
        .arg(format!("{0}", comm_vec.join(" ")))
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .spawn()
        .expect("Problem running the command");

    let (send_print, recv_print) = sync_channel(1);
    let (send_finished, recv_finished) = sync_channel(1);

    thread_read_stdxxx(
        send_print.clone(),
        comm.stdout.expect("Error getting stdout"),
    );
    thread_read_stdxxx(
        send_print.clone(),
        comm.stderr.expect("Error getting stderr"),
    );
    thread_send_print_elapsed_time(send_print.clone(), recv_finished);
    drop(send_print);

    let process_print = |status, print| match print {
        Print::ElapsedTime => print_elapsed_time(status),
        Print::Line(line) => print_line(&line, status),
        Print::FinishedTasks => finished_task(status),
    };

    recv_print
        .iter()
        .fold(Status::init(send_finished), |status, received| {
            process_print(status, received).check_exit()
        });

    println!("\n");
}


fn thread_read_stdxxx<OutErr>(sender: SyncSender<Print>, out_err: OutErr)
where
    OutErr: std::io::Read + std::marker::Send + std::marker::Sync + 'static,
{
    thread::spawn(move || {
        let child_buf = std::io::BufReader::new(out_err);
        for line in child_buf.lines() {
            let _ = sender.send(Print::Line(line.unwrap_or("".to_owned())));
            let _ = sender.send(Print::ElapsedTime);
        }
        let _ = sender.send(Print::FinishedTasks);
    });
}


fn thread_send_print_elapsed_time(sender: SyncSender<Print>, recv_finished: Receiver<()>) {
    thread::spawn(move || loop {
        match recv_finished.recv_timeout(Duration::from_millis(250)) {
            Ok(_) => break,
            Err(_) => {
                let _ = sender.send(Print::ElapsedTime);
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

fn finished_task(mut status: Status) -> Status {
    status.finished_tasks += 1;
    status
}


fn get_string_time(total_seconds: u64) -> String {
    let div_rem = |dividend, divisor| (dividend / divisor, dividend % divisor);
    let (total_minuts, seconds) = div_rem(total_seconds, 60);
    match total_minuts > 0 {
        true => format!("{}m{:02}s", total_minuts, seconds),
        false => format!("{}s", seconds),
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
