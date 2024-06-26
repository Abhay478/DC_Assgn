#include <zmq.hpp>
#include <zmq.h>
#include <iostream>
#include <vector>
#include <fstream>
#include <utility>
#include <string>
#include <sstream>
#include <future>
#include <chrono>
#include <random>
#include <signal.h>
#include <unistd.h>
#include <stdio.h>
#include <execinfo.h>

using namespace std;
std::random_device rd;
std::mt19937 gen(rd());

string ip;

string get_addr(int i) {
    return "tcp://" + ip + ":" + to_string(i + 5550); // magic
    // return "inproc://" + to_string(i + 5550);
}

chrono::system_clock::time_point init;

chrono::system_clock::duration get_time() {
    return (chrono::system_clock::now() - init) / 1000;
}

// Signal handling for debugging
void handle(int sig) {
    void *array[20];
    size_t size;

    // get void*'s for all entries on the stack
    size = backtrace(array, 20);

    // print out all the frames to stderr
    fprintf(stderr, "Error: signal %d:\n", sig);
    backtrace_symbols_fd(array, size, STDERR_FILENO);
    exit(1);
}

// One entry in the log
struct LogEntry {
    chrono::system_clock::duration time;
    string event;
    vector<int> clock;
    int space;
    int tid;
    int other_id;
    LogEntry(vector<int> clock, chrono::system_clock::duration time, string event, int space, int tid, int other_id) 
    : time(time), event(event), clock(clock), tid(tid), space(space), other_id(other_id) {}

    void print(ostream &f) {
        if(event == "tick") {
            f << "Process" << tid << " executes internal event e" << tid << "_" << clock[tid] << " at " << time.count() << ", vc: [";
        } else if (event == "send") {
            f << "Process" << tid << " sends message m" << tid << "_" << other_id << " to process" << other_id << " at " << time.count() << ", vc: [";
        } else if (event == "recv") {
            f << "Process" << tid << " receives message m" << other_id << "_" << tid << " from process" << other_id << " at " << time.count() << ", vc: [";
        } else if (event == "term_send") {
            f << "Process" << tid << " sends termination message to process" << other_id << " at " << time.count() << ", vc: [";
        } else if (event == "term_recv") {
            f << "Process" << tid << " receives termination message at " << time.count() << ", vc: [";
        }
        for(auto &c : clock) {
            f << c << ", ";
        }
        f << "]" << endl;
        // f << "Process " << tid << " " << event << " at " << time.count() << " with clock: ";
        
    }
};

// The entire log for a thread
struct Log {
    // Redundant, but convenient.
    int tid;
    vector<LogEntry *> entries;
    Log(int tid) : tid(tid) {}
    void log(vector<int> &clock, chrono::system_clock::duration time, string event, int space = 0, int other = -1) {
        entries.push_back(new LogEntry(clock, time, event, space, tid, other));
    }

    int msg_space() {
        int sum = 0;
        for(auto &e : entries) {
            if(e->event == "send") {
                sum += e->space;
            }
        }
        return sum;
    }
    

    void print(ostream &f) {
        for(auto &e : entries) {
            e->print(f);
        }
    }
};

// Input parameters
struct Params {
    int n;
    exponential_distribution<> exp;
    bernoulli_distribution bern;
    int m;

    Params(fstream &f) {
        double l, a;
        f >> n >> l >> a >> m;
        exp = exponential_distribution<>(1/l);
        bern = bernoulli_distribution(1/(1+a));
    }
};

// Graph node
struct Node {
    int node_id;
    vector<pair<int, zmq::socket_t *>> socks;
    zmq::socket_t * recvr;
    vector<int> vtime;
    uniform_int_distribution<> dist;

    Node(int n, zmq::context_t &ctx, int node_id) : node_id(node_id) {
        socks = vector<pair<int, zmq::socket_t *>>();
        recvr = new zmq::socket_t(ctx, ZMQ_PULL);
        recvr->bind(get_addr(node_id));
        vtime = vector<int>(n, 0);
    }

    void init_dist() {
        dist = uniform_int_distribution<>(0, socks.size() - 1);
    }

    virtual void recv_handler(Log * log, zmq::message_t &msg, int tid) {
        int * v = (int *)msg.data();
        log->log(vtime, get_time(), "recv", 0, v[msg.size() / sizeof(int) - 1]);
        // Rule 2.
        for(size_t i = 0; i < msg.size() / sizeof(int) - 1; i++) {
            if(i != tid) vtime[i] = max(vtime[i], v[i]);
        }
    }

    virtual void send_handler(Log * log, int tid) {
        int u = dist(gen);
        // vector<int> vtime_copy(vtime);
        // vtime_copy.insert(vtime_copy.begin(), tid);
        // Send to random neighbour
        vtime.push_back(tid);
        int space = vtime.size() * sizeof(int);
        zmq::message_t msg(space);
        memcpy(msg.data(), vtime.data(), space);
        socks[u].second->send(msg, zmq::send_flags::none);
        vtime.pop_back();
        log->log(vtime, get_time(), "send", space, socks[u].first);
    }

    Log * thread_fn(int tid, Params &params) {
        // Init thread log
        Log * log = new Log(tid);
        // Count of terminated neighbours
        size_t n_term = 0;
        int i = 0;
        while(i < params.m) {
            zmq::message_t msg;
            // Waiting on recv instead of `thread::sleep`ing.
            recvr->set(zmq::sockopt::rcvtimeo, (int)params.exp(gen)); 

            // Recv event?
            if (recvr->recv(msg, zmq::recv_flags::none)) {
                if(!msg.size()) { // Neighbour terminated
                    n_term++;
                    log->log(vtime, get_time(), "term_recv");
                    continue;
                }
                this->recv_handler(log, msg, tid);
                // msg.rebuild();
            } else if(params.bern(gen)) { // Send event?
                this->send_handler(log, tid);
                // cerr << "Send." << endl;
                i++;
            } else {
                log->log(vtime, get_time(), "tick");
            }
            // Normie event. Only rule 1.
            vtime[tid]++;
        }
        // Termination
        for(auto s: socks) {
            s.second->send(zmq::str_buffer(""), zmq::send_flags::none);
            log->log(vtime, get_time(), "term_send", 0, s.first);
            // vtime[tid]++;
            // Not a clocked event. Since we're not sending vector clocks in the message, we cannot increment the clock.
        }

        // Wait for all neighbours to terminate
        zmq::message_t msg;
        recvr->set(zmq::sockopt::rcvtimeo, 1000);
        while(n_term < socks.size()) {
            if(!recvr->recv(msg, zmq::recv_flags::none)) continue;
            if(!msg.size()) {
                n_term++;
                log->log(vtime, get_time(), "term_recv");
            }
            else {
                recv_handler(log, msg, tid);
            }
            vtime[tid]++;
        }
        return log;
    }
};

// Node, but with ls and lu
struct SKNode : public Node {
    vector<int> ls;
    vector<int> lu;
    SKNode(int n, zmq::context_t &ctx, int node_id) : Node(n, ctx, node_id), ls(n, 0), lu(n, 0) {}

    void recv_handler(Log * log, zmq::message_t &msg, int tid) override {
        // Rule 2.
        pair<int, int> * v = (pair<int, int> *)msg.data();
        log->log(vtime, get_time(), "recv", 0, v[msg.size() / sizeof(pair<int, int>) - 1].first);
        for(size_t i = 0; i < msg.size() / sizeof(pair<int, int>) - 1; i++) {
            if(vtime[v[i].first] < v[i].second) {
                vtime[v[i].first] = v[i].second;
                lu[v[i].first] = vtime[tid];
            }
        }
    }

    void send_handler(Log * log, int tid) override {
        vector<pair<int, int>> yeet;
        int rec = this->dist(gen); // Neighbour index, not node id.

        // Deltas
        lu[tid] = vtime[tid];
        for(size_t j = 0; j < vtime.size(); j++) {
            if(lu[j] >= ls[socks[rec].first]) { // node id
                yeet.push_back(make_pair(j, vtime[j]));
            }
        }
        yeet.push_back(make_pair(tid, 0));
        int space = yeet.size() * sizeof(pair<int, int>);
        log->log(vtime, get_time(), "send", space, socks[rec].first);
        socks[rec].second->send(zmq::buffer(yeet.data(), space));
        ls[socks[rec].first] = vtime[tid];
    }

    // Inherits a bunch of other functions.
};

// Initialise sockets for a pair of nodes
pair<zmq::socket_t *, zmq::socket_t *> get_pair(zmq::context_t *ctx, int i, int j) {
    string s1 = get_addr(i);
    string s2 = get_addr(j);

    auto out = make_pair(new zmq::socket_t(*ctx, ZMQ_PUSH), new zmq::socket_t(*ctx, ZMQ_PUSH));
    out.first->connect(s2);
    out.second->connect(s1);
    return out;
}

/// @brief Need this function coz can't spawn a thread on a method.
/// @param node 
/// @param params 
/// @param tid 
template <typename T>
Log * thread_init(T * node, Params * params, int tid) {
    return node->thread_fn(tid, *params);
}

// Big graph. Each Graph instance has it's own context. The Graph can contain nodes of any kind, although they must inherit Node to have the proper methods.
template <typename T>
struct Graph {
    vector<T> nodes;
    zmq::context_t zmq_ctx;

    Graph(fstream &f, int n) {
        // static_assert(std::is_base_of<Node, T>::value, "T must inherit from Node");
        zmq_ctx = zmq::context_t(1);
        // nodes.reserve(n);
        for(int i = 0; i < n; i++) {
            nodes.push_back(T(n, zmq_ctx, i));
        }
        char * line = new char[100];
        while(f.getline(line, 100)) {
            string s(line);
            if(s == "") {
                continue;
            }
            stringstream ss(s);
            int i;
            ss >> i;
            int j;
            while(ss >> j) {
                // i can send to j
                string addr = get_addr(j - 1);
                // cerr << addr << endl;
                zmq::socket_t * new_sock = new zmq::socket_t(zmq_ctx, ZMQ_PUSH);
                new_sock->connect(addr);
                nodes[i - 1].socks.push_back({j - 1, new_sock});
                // cerr << "Connected " << i << " to " << j << endl;
            }
        }

        for(auto &n : nodes) {
            n.init_dist();
        }

        f.close();
    }

    void thread_spawn(Params * params, fstream &f) {
        vector<future<Log *>> futures;
        for(int i = 0; i < params->n; i++) {
            futures.push_back(std::async(launch::async, thread_init<T>, &nodes[i], params, i));
        }

        vector<Log *> res;
        for(auto &t : futures) {
            res.push_back(t.get());
        }
        vector<LogEntry *> all_logs;
        double space = 0;
        for(auto &l : res) {
            space += l->msg_space();
            all_logs.insert(all_logs.end(), l->entries.begin(), l->entries.end());
        }

        space /= params->m * params->n;
        cout << space << endl;

        std::sort(all_logs.begin(), all_logs.end(), [](LogEntry * a, LogEntry * b) {
            return a->time < b->time;
        });
        for (auto &l : all_logs) {
            l->print(f);
        }
    }

};

// Context for the entire program. Contains the graph and the parameters.
template <typename T>
struct Context {
    Graph<T> * graph;
    Params * params;
    fstream in;
    fstream out;

    Context(string f_in, string f_out) {
        in.open(f_in, ios::in);
        out.open(f_out, ios::out);
        params = new Params(in);
        graph = new Graph<T>(in, params->n);
        in.close();
    }

    void thread_spawn() {
        graph->thread_spawn(params, out);
    }
};

