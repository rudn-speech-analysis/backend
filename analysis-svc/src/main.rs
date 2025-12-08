mod whisper;

use std::{
    ffi::{CStr, CString},
    io::{Read, Write},
    os::fd::FromRawFd,
    sync::Mutex,
};

use axum::routing::get;
use ipc_channel::ipc::{IpcReceiver, IpcSender};
use libc::atexit;
use pyo3::{
    IntoPyObjectExt, PyResult, Python,
    types::{IntoPyDict, PyAnyMethods},
};

use crate::whisper::{FromChildMsg, FromParentMsg, worker};

fn run(code: &str) -> PyResult<()> {
    println!("Running: {code}");
    Python::attach(|py| {
        py.run(&CString::new(code).unwrap(), None, None).unwrap();
        PyResult::Ok(())
    })?;
    println!("Run finished");
    Ok(())
}

fn try_thread(recursion: bool) {
    let (tx, rx) = ipc_channel::ipc::channel::<String>().unwrap();
    Python::initialize();

    match fork::fork() {
        Ok(fork::Fork::Parent(pid)) => {
            println!("I'm parent, my child has pid: {pid}");
            let recv = rx.recv().unwrap();
            println!("Received: {recv}");
            if recursion {
                std::thread::sleep(std::time::Duration::from_secs(10));
                try_thread(false);
            }
        }
        Ok(fork::Fork::Child) => {
            println!("I'm child");
            run("print('hello from child')").unwrap();
            run("print('is os imported?', 'os' in globals())").unwrap();
            run("import os").unwrap();
            run("print('is os imported?', 'os' in globals())").unwrap();
            run("print('child pid is:', os.getpid())").unwrap();
            tx.send("hello, parent!".to_string()).unwrap();
        }
        Err(_) => todo!(),
    }
}

struct WorkerHandle {
    pid: i32,
    tx: IpcSender<FromParentMsg>,
    rx: IpcReceiver<FromChildMsg>,
}

fn spawn_worker(arr: &mut Vec<WorkerHandle>) {
    let (child_tx, parent_rx) = ipc_channel::ipc::channel::<FromChildMsg>().unwrap();
    let (parent_tx, child_rx) = ipc_channel::ipc::channel::<FromParentMsg>().unwrap();
    match fork::fork() {
        Ok(fork::Fork::Parent(pid)) => {
            println!("I'm parent, my child has pid: {pid}");
            arr.push(WorkerHandle {
                pid,
                tx: parent_tx,
                rx: parent_rx,
            });
        }
        Ok(fork::Fork::Child) => {
            worker(child_tx, child_rx);
        }
        Err(_) => todo!(),
    }
}

static mut PIDS_TO_KILL: Mutex<Vec<i32>> = Mutex::new(Vec::new());

#[allow(static_mut_refs)]
extern "C" fn kill_all() {
    unsafe {
        let pids = PIDS_TO_KILL.lock().unwrap();
        for pid in pids.iter() {
            println!("Killing {pid}");
            libc::kill(*pid, libc::SIGTERM);
            let mut did_die = false;
            for _ in 0..10 {
                let result = fork::waitpid_nohang(*pid).unwrap();
                if result.is_some() {
                    println!("{pid} exited OK");
                    did_die = true;
                    break;
                } else {
                    std::thread::sleep(std::time::Duration::from_millis(100));
                }
            }
            if !did_die {
                println!("{pid} did not die from SIGTERM, sending SIGKILL");
                libc::kill(*pid, libc::SIGKILL);
            }
        }
    }
}

#[tokio::main]
#[allow(static_mut_refs)]
async fn main() -> eyre::Result<()> {
    let mut workers = Vec::new();
    spawn_worker(&mut workers);
    spawn_worker(&mut workers);

    let pids = workers.iter().map(|w| w.pid).collect::<Vec<i32>>();
    unsafe {
        *(PIDS_TO_KILL.lock().unwrap()) = pids;
    }
    unsafe { libc::atexit(kill_all) };

    // Python::initialize();
    // run("import whisper")?;
    // run("import json")?;
    // run("model = whisper.load_model(\"base\")")?;

    // let app = axum::Router::new().route("/", get(transcribe));

    // let listener = tokio::net::TcpListener::bind("0.0.0.0:3001").await.unwrap();

    // axum::serve(listener, app.into_make_service())
    //     .with_graceful_shutdown(async {
    //         tokio::signal::ctrl_c().await.unwrap();
    //     })
    //     .await
    //     .unwrap();

    Ok(())
}

async fn transcribe() -> String {
    let data = Python::attach(|py| {
        let locals = [(
            "path",
            "/tmp/Архив 4/mix_13012_13135__2025_10_01__09_18_59_590.mp3",
        )]
        .into_py_dict(py)
        .unwrap();
        py.eval(
            &CString::new("str(model.transcribe(path, verbose=True))").unwrap(),
            None,
            Some(&locals),
        )
        .unwrap()
        .extract()
    })
    .unwrap();

    data
}
