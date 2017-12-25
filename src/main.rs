// TODO
//  colors

// DONE
//  remove for loop
//  time format h:mm:ss
//  add help
//  avoid printing time on same second
//  force print time at end

extern crate termion;

use std::io::BufRead;
use std::process::{Command, Stdio};
use std::time::Instant;
use std::time::Duration;
use std::thread;
use std::sync::mpsc::{sync_channel, Receiver, SyncSender};


enum Print {
    Line(String),
    ElapsedTime,
    FinishedTasks,
}

#[derive(Clone)]
struct Seconds(u64);

#[derive(Clone, PartialEq)]
enum LastLine {
    Time,
    Output,
}

#[derive(Clone)]
struct Status {
    last_line: LastLine,
    prev_seconds: Seconds,
    finished_tasks: u8,
    sender_finished: SyncSender<()>,
}

impl Status {
    fn init(send_finished: SyncSender<()>) -> Status {
        Status {
            last_line: LastLine::Output,
            prev_seconds: Seconds(0),
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
        .unwrap();

    let start = Instant::now();

    let (send_print, recv_print) = sync_channel(1);
    let (send_finished, recv_finished) = sync_channel(1);

    thread_read_stdxxx(send_print.clone(), comm.stdout.unwrap());
    thread_read_stdxxx(send_print.clone(), comm.stderr.unwrap());
    thread_send_print_elapsed_time(send_print.clone(), recv_finished);
    drop(send_print);

    let process_print = |status, print| match print {
        Print::ElapsedTime => print_elapsed_time(&start, status, false),
        Print::Line(line) => print_line(&line, &start, status),
        Print::FinishedTasks => finished_task(status),
    };

    recv_print
        .iter()
        .fold(Status::init(send_finished), |status, received| {
            process_print(status, received).check_exit()
        });

    print_total_time(&start);
    println!("");
}


fn thread_read_stdxxx<OutErr>(sender: SyncSender<Print>, out_err: OutErr)
where
    OutErr: std::io::Read + std::marker::Send + std::marker::Sync + 'static,
{
    thread::spawn(move || {
        let child_buf = std::io::BufReader::new(out_err);
        for line in child_buf.lines() {
            let _ = sender.send(Print::Line(line.unwrap()));
            let _ = sender.send(Print::ElapsedTime);
        }
        let _ = sender.send(Print::FinishedTasks);
    });
}


fn thread_send_print_elapsed_time(sender: SyncSender<Print>, recv_finished: Receiver<()>) {
    thread::spawn(move || {
        loop {
            match recv_finished.recv_timeout(Duration::from_millis(500)) {
                Ok(_) => break,
                Err(_) => {
                    let _ = sender.send(Print::ElapsedTime);
                }
            }
        }
    });
}

fn print_elapsed_time(start: &Instant, mut status: Status, force: bool) -> Status {
    use termion::clear;
    use termion::cursor::{self, DetectCursorPos};
    use termion::input::MouseTerminal;
    use termion::raw::IntoRawMode;
    use std::io::{self, Write};

    let mut stdout = MouseTerminal::from(io::stdout().into_raw_mode().unwrap());
    let (_, y) = stdout.cursor_pos().unwrap();
    let seconds = Seconds(start.elapsed().as_secs());
    if status.prev_seconds.0 != seconds.0 || force {
        let _ = write!(
            stdout,
            "{}{}  ---    [{}]    ---",
            clear::CurrentLine,
            cursor::Goto(1, y),
            get_string_time(&seconds)
        );
        let _ = stdout.flush();
        status.last_line = LastLine::Time;
    }
    status.prev_seconds = seconds;
    status
}

fn print_line(line: &str, start: &Instant, mut status: Status) -> Status {
    if status.last_line == LastLine::Time {
        println!("");
    }
    println!("{}", line);
    status.last_line = LastLine::Output;
    print_elapsed_time(start, status, true)
}

fn finished_task(mut status: Status) -> Status {
    status.finished_tasks += 1;
    status
}

fn print_total_time(start: &Instant) {
    println!(
        "\n>>>  Total time: {}  <<<",
        get_string_time(&Seconds(start.elapsed().as_secs()))
    );
}

fn get_string_time(total_secs: &Seconds) -> String {
    let div_rem = |dividend, divisor| (dividend / divisor, dividend % divisor);
    let (total_minuts, seconds) = div_rem(total_secs.0, 60);
    let (total_hours, minuts) = div_rem(total_minuts, 60);

    format!("{}:{:02}:{:02}", total_hours, minuts, seconds)
}
