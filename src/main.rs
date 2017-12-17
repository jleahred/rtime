// pending stderr
// using args for command
//

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
    Finished,
}


fn main() {
    let comm = Command::new("sh")
        .arg("-c")
        .arg("for i in $(seq 1 3); do sleep 3; echo line $i; done")
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .spawn()
        .unwrap();

    let start = Instant::now();

    let (sender, receiver) = sync_channel(1);

    let sender_elapsed_time = sender.clone();
    thread::spawn(move || {
        loop {
            thread::sleep(Duration::from_millis(500));
            let _ = sender_elapsed_time.send(Print::ElapsedTime);
        }
    });

    let sender_stdout = sender.clone();
    thread::spawn(move || {
        let child_buf = std::io::BufReader::new(comm.stdout.unwrap());
        for line in child_buf.lines() {
            let _ = sender_stdout.send(Print::Line(line.unwrap()));
            let _ = sender_stdout.send(Print::ElapsedTime);
        }
        let _ = sender_stdout.send(Print::Finished);
    });

    for print in receiver.iter() {
        match print {
            Print::ElapsedTime => print_elapsed_time(&start),
            Print::Line(line) => println!("\n{}", line),
            Print::Finished => break,
        }
    }

    println!("");
}

fn print_elapsed_time(start: &Instant) {
    use termion::clear;
    use termion::cursor::{self, DetectCursorPos};
    use termion::input::MouseTerminal;
    use termion::raw::IntoRawMode;
    use std::io::{self, Write};

    let mut stdout = MouseTerminal::from(io::stdout().into_raw_mode().unwrap());
    let (_, y) = stdout.cursor_pos().unwrap();
    let _ = write!(stdout,
                   "{}{}[{}s]",
                   clear::CurrentLine,
                   cursor::Goto(1, y),
                   start.elapsed()
                       .as_secs());
    let _ = stdout.flush();
}