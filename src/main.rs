// TODO
//  add help
//  time format hh:mm:ss
//  colors
//  remove for loop

// DONE
//  avoid printing time on same second
//  force print time at end

extern crate termion;

use std::io::BufRead;
use std::process::{Command, Stdio};
use std::time::Instant;
use std::time::Duration;
use std::thread;
use std::sync::mpsc::sync_channel;

enum Print {
    Line(String),
    ElapsedTime,
    FinishedTasks,
}

#[derive(Clone)]
struct Seconds(u64);

#[derive(Clone)]
enum LastLine {
    Time,
    Output,
}

#[derive(Clone)]
struct Status {
    last_line: LastLine,
    prev_seconds: Seconds,
    finished_tasks: u8,
}

fn main() {
    if std::env::args().count() == 1 {
        panic!("missing command to execute");
    }

    let comm_vec: std::vec::Vec<_> = std::env::args().skip(1).collect();
    let comm = Command::new("sh")
        .arg("-c")
        //.arg("for i in $(seq 1 3); do sleep 3; echo line $i; done")   //  to test
        .arg(comm_vec.join(" "))
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .spawn()
        .unwrap();

    let start = Instant::now();

    let (sender, receiver) = sync_channel(1);

    thread_send_print_elapsed_time(sender.clone());
    thread_read_stdxxx(sender.clone(), comm.stdout.unwrap());
    thread_read_stdxxx(sender.clone(), comm.stderr.unwrap());


    let mut status = Status {
        last_line: LastLine::Output,
        prev_seconds: Seconds(0),
        finished_tasks: 0,
    };
    for print in receiver.iter() {
        status = match print {
            Print::ElapsedTime => print_elapsed_time(&start, &status),
            Print::Line(line) => {
                println!("{}", line);
                status.last_line = LastLine::Output;
                status
            }
            Print::FinishedTasks => {
                status.finished_tasks += 1;
                status
            }
        };
        if status.finished_tasks == 2 {
            break;
        }
    }

    print_total_time(&start);
    println!("");
}


fn thread_read_stdxxx<OutErr>(sender: std::sync::mpsc::SyncSender<Print>, out_err: OutErr)
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


fn thread_send_print_elapsed_time(sender: std::sync::mpsc::SyncSender<Print>) {
    thread::spawn(move || {
        loop {
            thread::sleep(Duration::from_millis(500));
            let _ = sender.send(Print::ElapsedTime);
        }
    });
}

fn print_elapsed_time(start: &Instant, status: &Status) -> Status {
    use termion::clear;
    use termion::cursor::{self, DetectCursorPos};
    use termion::input::MouseTerminal;
    use termion::raw::IntoRawMode;
    use std::io::{self, Write};

    let mut result = status.clone();
    let mut stdout = MouseTerminal::from(io::stdout().into_raw_mode().unwrap());
    let (_, y) = stdout.cursor_pos().unwrap();
    let seconds = start.elapsed().as_secs();
    if status.prev_seconds.0 != seconds {
        let _ = write!(
            stdout,
            "{}{}[{}s]",
            clear::CurrentLine,
            cursor::Goto(1, y),
            seconds
        );
        let _ = stdout.flush();
        result.last_line = LastLine::Time;
    }
    result.prev_seconds = Seconds(seconds);
    result
}

fn print_total_time(start: &Instant) {
    println!("\n>>>  Total time: {}  <<<", start.elapsed().as_secs());
}
