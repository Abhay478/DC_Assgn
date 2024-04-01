---
title: "Assignment 2"
author: "Abhay Shankar K: cs21btech11001"
geometry: margin=2cm
---

# Structs

## Params

Fields as per input format: `n`, `k`, `a`, `b`. 

### Methods

- `sleep`: Simulates computation both inside and outside the CS by sleeping.

## MaekawaNode

### Fields

- `id`: Coordinates of the node in the grid.
- `ips`: A map from node IDs to their IP addresses.
- `rx`: A socket to listen for incoming requests.
- `init`: Start time.
- `seq`: Lamport clock.

### Methods

- `spawn`: Spawns listener and requester threads. Collects log entries and prints to file.
- `requester_thread`: Enters CS `k` times.
- `listener_thread`: Acquires a poller and listens for events.
- `get_(listener|requester)_poller`: Returns a poller for the respective thread.
- `enter_cs`: Enters the critical section.
- `exit_cs`: Exits the critical section.
- `listen`: Listens for incoming messages and responds accordingly. 

## RCNode

### Fields

- `id`: Node ID.
- `ips`: A map from node IDs to their IP addresses.
- `rx`: A socket to listen for incoming requests.
- `init`: Start time.
- `seq`: Lamport clock.
- `req_flag`: Flag to indicate if the node is requesting to enter the CS.
- `quorum`: Flags indicating whether a request or a reply should be sent to a given node. 

### Methods

Methods are similar to `MaekawaNode`.

## Message

### Fields

- `id`: Node ID of either `MaeakwaNode` or `RCNode`.
- `typ`: Type of message.
- `ts`: Lamport clock.

## LogEntry

### Fields

- `pid`: Node ID of either `MaeakwaNode` or `RCNode`.
- `ts`: Time of the log entry.
- `act`: Type of event being logged.

## Request

Struct for pending requests in a `MaekawaNode` listener.

### Fields

- `pid`: Node ID of `MaekawaNode`.
- `ts`: Lamport clock.
- `stream`: Connection to the requesting node.


# Utilities

- `get_ips`: Reads IP addresses from a file.
- `get_a_stream`: Returns a connection to a socket.
- `get_msgs`: Returns a vector of `Message`s parsed from a byte array.

# Files

- `src`
  - `bin`
    - `q1.rs`: Creates a Maekawa node process.
    - `q2.rs`: Creates an RC node process.
  - `lib.rs`: Module root.
  - `maekawa.rs`: Contains the `MaekawaNode` struct.
  - `rc.rs`: Contains the `RCNode` struct.
  - `utils.rs`: Contains utility functions.
  - `request.rs`: Contains the `Request` struct.
- `log`
  - `maekawa`
    - One log file per node.
  - `rc`
    - One log file per node.
- `inp-params.txt`: Input parameters.
- `ips.txt`: IP addresses of nodes.
- `Cargo.*`: Rust manifest files.

# Program flow

- Whenever a new process is created, it must be provided its process ID (aka node ID) as a command-line argument.
- It reads the input parameters from `inp-params.txt` and creates an instance of the appropriate struct.
- The process binds to the corresponding IP address and port number in the `ips.txt` file. 
- Two threads are spawned, one responsible for handling incoming requests and the other for generating new ones, in the `listener_thread` and `requester_thread` methods respectively.
- After the execution of the algorithm, the logs are collected and written to a file.

# Execution instructions

Use the command

```sh
cargo r --release -q --bin q1 -- (node ID here)
```

- This command will run the Maekawa algorithm. Replace `q1` with `q2` to run the RC algorithm.
- This command creates one node. To create more, run the command multiple times with different node IDs. Giving a duplicate node ID will result in an error.

# Graphs

TODO