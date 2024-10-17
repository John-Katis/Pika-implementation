# README

## Pika Implementation

In this repository all the necessary functionality around implementing the semi-honest protocol of Pika is given in Rust. Using the existing functions and some of the given steps, your task is to:

`MAIN`: Implement the pika semi-honest protocol for different non-linear functions (TBD)

* `MAIN: Offline Phase`: generate required randomness and shares of input and beaver triples (steps 0 & 1 of the semi-honest protocol)
* `MAIN: Online Phase`: implement the steps 2 & 3 of the semi-honest protocol from the paper

`SECONDARY`: Derive some benchmarking statistics for each non-linear function

This README further contains some useful information on:

* [How to download and install rust](#download--install-rust)
* [The structure/contents of this Repository](#repository-structure)
* [Which files you will need to work on](#your-task)
* [How to run the communication between server and client](#running-the-code)

---
### Download / Install Rust

You can find the distributable for Rust [here](https://www.rust-lang.org/tools/install). This link contains helpful documentation too. For the installation, you simply need to follow the instructions that are provided through the command line.

This [link](https://www.rust-lang.org/learn) also provides helpful material, including a guide for downloading and setting up rust. No need to spend too much time here, but there may be some useful information.

---
### Repository structure

This repository contains 5 folders, 2 of which are meant to only carry data (input and output) files (in whichever format, e.g., .bin or .txt). Those are:

* `data`: helper folder for data generated in the offline phase
* `input`: contains a single file with the input for the computation

The rest of the folders contain the following:

* `frontend`: This is the folder that contains the application endpoint (meaning you should run the code from within this folder. This is explained in [Running the code](#running-the-code).)
Contains also the code file `main.rs`. This contains the code that creates the parties and performs the offline and online phases of the protocol.
* `libfss`: Here all necessary implementations for (1) beaver triples, (2) DPF keys, (3) prg seed and (4) ring elements, are given. You can check out the code to figure out how to use it in the protocol implementation.
* `libmpc`: This one contains the code files for the (1) parties instantiation & communication, (2) offline phase generation and (3) the pika online protocol (in `src/protocols`).

All folders have a `src` and `target` subfolde. `target` will contain the compiled code files and `src` always has the code files that need to be worked on. `Cargo.lock` or `.toml` are configuration files that are configured already and will work from the start of the project.

---
### Your task

For your task, you will have to work on `frontend/src/main.rs` and `libmpc/src` files. Specifically, you will have to work on the following files for the different tasks of the project:

* For the **offline phase** you will need to work in `libmpc/src/offline_data.rs`.
* For the **online phase** you will need to work in `libmpc/src/protocols/pika.rs` and `frontend/src/main.rs`.
* For the secondary task of **benchmarking**, you would have to make changes to `libmpc/src/offline_data.rs`,  `frontend/src/main.rs` and `libmpc/src/mpc_platform.rs`.

---
### Running the code

In order to run the code you need to be in the frontend folder of the repository in 2 open terminals and run:

```bash
    C:\Users\IKatis\Desktop\PIKA\frontend> cargo run 0
```

to run the server and 

```bash
    C:\Users\IKatis\Desktop\PIKA\frontend> cargo run 1
```

to run the client. Notice that both terminals are in the same folder and only the number of the call you make changes. Once both are run (and the code has no errors) the protocol will take place.

If there are errors in the code, those are caught after you run `cargo run 0`, because this command combines the compilation of your code and the running of the application simultaneously. If you wish to **simply compile** your code to check for errors, you can use the following command while being the in frontend folder:

```bash
    C:\Users\IKatis\Desktop\PIKA\frontend> cargo build
```

This will only compile the code and not run the application.

[BACK TO TOP](#pika-implementation)
