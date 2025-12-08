use std::{ffi::CString, os};

use ipc_channel::ipc::{IpcReceiver, IpcSender};
use pyo3::{
    PyResult, Python,
    types::{IntoPyDict, PyAnyMethods},
};
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum FromParentMsg {
    Transcribe { path: String },
    Exit,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FromChildMsg {
    data: String,
}

fn run(code: &str) -> PyResult<()> {
    Python::attach(|py| {
        py.run(&CString::new(code).unwrap(), None, None)?;
        PyResult::Ok(())
    })?;
    Ok(())
}

pub fn worker(tx: IpcSender<FromChildMsg>, rx: IpcReceiver<FromParentMsg>) {
    Python::initialize();
    // run("import whisper").unwrap();
    // run("import json").unwrap();
    // run("model = whisper.load_model(\"base\")").unwrap();
    loop {
        let Ok(command) = rx.recv() else {
            println!("Child process's recv failed");
            std::process::exit(0)
        };
        match command {
            FromParentMsg::Exit => break,
            FromParentMsg::Transcribe { path } => {
                let data: String = Python::attach(|py| {
                    let locals = [("path", path)].into_py_dict(py).unwrap();
                    py.eval(
                        &CString::new("str(model.transcribe(path, verbose=True))").unwrap(),
                        None,
                        Some(&locals),
                    )
                    .unwrap()
                    .extract()
                })
                .unwrap();
                tx.send(FromChildMsg { data }).unwrap();
            }
        }
    }
}
