// add help
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


    for print in receiver.iter() {
        match print {
            Print::ElapsedTime => print_elapsed_time(&start),
            Print::Line(line) => println!("\n{}", line),
            Print::Finished => break,
        }
    }

    println!("");
}


fn thread_read_stdxxx<OutErr>(sender: std::sync::mpsc::SyncSender<Print>, stdout: OutErr)
where
    OutErr: std::io::Read + std::marker::Send + std::marker::Sync + 'static,
{
    thread::spawn(move || {
        let child_buf = std::io::BufReader::new(stdout);
        for line in child_buf.lines() {
            let _ = sender.send(Print::Line(line.unwrap()));
            let _ = sender.send(Print::ElapsedTime);
        }
        let _ = sender.send(Print::Finished);
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

fn print_elapsed_time(start: &Instant) {
    use termion::clear;
    use termion::cursor::{self, DetectCursorPos};
    use termion::input::MouseTerminal;
    use termion::raw::IntoRawMode;
    use std::io::{self, Write};

    let mut stdout = MouseTerminal::from(io::stdout().into_raw_mode().unwrap());
    let (_, y) = stdout.cursor_pos().unwrap();
    let _ = write!(
        stdout,
        "{}{}[{}s]",
        clear::CurrentLine,
        cursor::Goto(1, y),
        start.elapsed().as_secs()
    );
    let _ = stdout.flush();
}
