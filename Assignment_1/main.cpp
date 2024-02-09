#define ZMQ_HAVE_POLLER
#include <zmq.hpp>
#include <zmq.h>
#include <iostream>
#include <string_view>
#include <vector>
#include <fstream>
#include <utility>
#include <string>
#include <sstream>
#include <thread>
#include <chrono>
#include <random>
#include <signal.h>
#include <unistd.h>
#include <stdio.h>
#include <execinfo.h>

using namespace std;
std::random_device rd;
std::mt19937 gen(rd());

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


struct Params {
    int n;
    // int l;
    // int a;
    exponential_distribution<> exp;
    bernoulli_distribution bern;
    int m;

    Params(fstream &f) {
        double l, a;
        f >> n >> l >> a >> m;
        // cout << n << " " << l << " " << a << " " << m << endl;
        exp = exponential_distribution<>(1/l);
        bern = bernoulli_distribution(1/(1+a));

        // for(int i = 0; i < 10; i++) {
        //     cout << exp(gen) << " " << bern(gen) << endl;
        // }
    }
};


struct Node {
    // zmq::socket_t sock;
    // zmq::socket_t pub;
    // zmq::socket_t sub;

    vector<zmq::socket_t *> socks;
    vector<int> vtime;
    uniform_int_distribution<> dist;
    // zmq::poller_t<> poller;
    vector<zmq::pollitem_t> poll_items;

    Node(int n) {
        // sock = zmq::socket_t(*ctx, ZMQ_DEALER);
        // pub = zmq::socket_t(*ctx, ZMQ_PUB);
        // sub = zmq::socket_t(*ctx, ZMQ_SUB);
        // sub.set(zmq::sockopt::subscribe, ""); // Subscribe to all topics.
        // sock.set(zmq::sockopt::router_mandatory, 1);
        // sock.set(zmq::sockopt::rcvtimeo, 5000);
        // sock.setsockopt(ZMQ_IDENTITY, addr.c_str(), addr.size());
        // pub.bind(addr);

        // sock.bind(addr);
        
        socks = vector<zmq::socket_t *>();
        vtime = vector<int>(n, 0);
        dist = uniform_int_distribution<>(0, n - 1);
        poll_items = vector<zmq::pollitem_t>();
    }

    int poll(int timeout) {
        // for(auto q: socks) {
        //     cout << "In poll" << ((zmq::socket_t *)q.socket)->connected() << endl;
        // }
        cout << "Polling" << endl;
        auto out = zmq::poll(poll_items, chrono::duration<int, milli>(timeout));
        if(out < 0) {
            cout << "Poll failed" << endl;
        }
        else {
            cout << "Poll succeeded" << endl;
        }

        return out;
    }

    // zmq::socket_t * get_sock(int i) {
    //     return (zmq::socket_t *)socks[i].socket;
    // }

    // zmq::socket_t random_sock() {
    //     return socks[dist(gen)];
    // }

    // ZMQ bullshit: poll doesn't return the index of the socket that has data, but the **number** of sockets that have.
    // Which is such a comically absurd thing to do, I can't even.
    void recv(zmq::message_t &msg) {
        int idx = -1;
        for(int j = 0; j < socks.size(); j++) {
            if(poll_items[j].revents & ZMQ_POLLIN) {
                idx = j;
                break;
            }
        }
        if(idx == -1) {
            cout << "No socket has data" << endl;
            return;
        }
        socks[idx]->recv(msg, zmq::recv_flags::none);
    }

    void thread_fn(int tid, Params &params) {
        for(int i = 0; i < params.m; i++) {
            zmq::message_t msg;
            // sub.set(zmq::sockopt::rcvtimeo, (int)params.exp(gen)); // TODO: Granularity


            // Recv event?
            // if (sub.recv(msg, zmq::recv_flags::none)) {
            //     // Logging, 
                
            //     // Rule 2.
            //     int * v = (int *)msg.data();
            //     for(int i = 0; i < msg.size() / sizeof(int); i++) {
            //         vtime[i] = max(vtime[i], v[i]);
            //     }
                
            // } else if(params.bern(gen)) { // Send event?
            //     pub.send(zmq::buffer(vtime.data(), vtime.size() * sizeof(int)));
            // }
            try {
                cout << "Polling " << tid << endl;
                // poll all sockets
                if (poll((int)params.exp(gen)) > 0) {
                    cout << "Recved something" << endl;
                    // recv'd something
                    // Logging, TODO
        
                    
                    recv(msg);

                    // Rule 2.  
                    int * v = (int *)msg.data();
                    for(int i = 0; i < msg.size() / sizeof(int); i++) {
                        vtime[i] = max(vtime[i], v[i]);
                    }
                } else if(params.bern(gen)) {
                    // Send message to random socket
                    cout << "Sending message" << endl;
                    socks[dist(gen)]->send(zmq::buffer(vtime.data(), vtime.size() * sizeof(int)));
                }
                // Normie event. Only rule 1.
                cout << "Incremented vtime" << endl;
                vtime[tid]++;
            } catch (exception &e) {
                for(auto s : socks) {
                    cout << s->get(zmq::sockopt::last_endpoint) << endl;
                }
                // poll(10);
                zmq::poll(&poll_items[0], poll_items.size(), chrono::duration<int, milli>(100));
                cout << e.what() << endl;
            }
        }
    }
};

struct SKNode : public Node {
    vector<int> ls;
    vector<int> lu;
    SKNode(zmq::context_t * ctx, std::string addr, int n) : Node(n), ls(n, 0), lu(n, 0) {}

    void thread_fn(int tid, Params &params) {
        for(int i = 0; i < params.m; i++) {
            zmq::message_t msg;
            /* sub.set(zmq::sockopt::rcvtimeo, (int)params.exp(gen)); // TODO: Granularity

            // Recv event?
            if (sub.recv(msg, zmq::recv_flags::none)) {
                // Logging, 
                
                // Rule 2.
                pair<int, int> * v = (pair<int, int> *)msg.data();
                vector<pair<int, int>> vec(v, v + msg.size() / sizeof(int));

                for (auto &p : vec) {
                    if(vtime[p.first] < p.second) {
                        vtime[p.first] = p.second;
                        lu[p.first] = vtime[tid];
                    }
                }
            } else if(params.bern(gen)) { // Send event?
                auto yeet = vector<pair<int, int>>();

                for(int j = 0; j < params.n; j++) {
                    if(lu[j] > ls) {
                        yeet.push_back(make_pair(j, vtime[j]));
                    }
                }
                pub.send(zmq::buffer(yeet.data(), yeet.size() * sizeof(int) * 2));
                ls = vtime[tid];
            }
            // Normie event. Only rule 1.
            vtime[tid]++; */
            // poll all sockets
            if (poll((int)params.exp(gen)) >= 0) {
                // recv'd something
                // Logging, TODO
                
                recv(msg);

                // Rule 2.
                pair<int, int> * v = (pair<int, int> *)msg.data();
                vector<pair<int, int>> vec(v, v + msg.size() / sizeof(int));

                for (auto &p : vec) {
                    if(vtime[p.first] < p.second) {
                        vtime[p.first] = p.second;
                        lu[p.first] = vtime[tid];
                    }
                }
            } else if(params.bern(gen)) {
                // Send message to random socket
                // random_sock()->send(zmq::buffer(vtime.data(), vtime.size() * sizeof(int)));
                auto yeet = vector<pair<int, int>>();
                int rec = dist(gen);
                for(int j = 0; j < params.n; j++) {
                    if(lu[j] > ls[rec]) {
                        yeet.push_back(make_pair(j, vtime[j]));
                    }
                }
                socks[rec]->send(zmq::buffer(yeet.data(), yeet.size() * sizeof(int) * 2));
                ls[rec] = vtime[tid];
            }
            // Normie event. Only rule 1.
            vtime[tid]++;
        }
    }
};

string get_addr(int i) {
    return "inproc://sock" + to_string(i + 5550); // magic
}

pair<zmq::socket_t *, zmq::socket_t *> get_pair(zmq::context_t *ctx, int i, int j) {
    string stub = "inproc://pair_";
    // return make_pair(stub + to_string(i) + "_" + to_string(j), stub + to_string(j) + "_" + to_string(i));
    string s1 = stub + to_string(i) + "_" + to_string(j);
    string s2 = stub + to_string(j) + "_" + to_string(i);

    auto out = make_pair(new zmq::socket_t(*ctx, ZMQ_PAIR), new zmq::socket_t(*ctx, ZMQ_PAIR));
    out.first->bind(s1);
    out.second->bind(s2);
    out.first->connect(s2);
    out.second->connect(s1);

    // out.first.send(zmq::str_buffer("Hi."), zmq::send_flags::none);
    // out.second.send(zmq::str_buffer("Hello."), zmq::send_flags::none);

    // auto msg = zmq::message_t();
    // auto r1 = out.first.recv(msg, zmq::recv_flags::none);
    // cout << "Recved " << msg.to_string() << endl;
    // auto r2 = out.second.recv(msg, zmq::recv_flags::none);

    // if(!r1 || !r2) {
    //     cout << "Failed to connect" << endl;
    // }
    // cout << "Recved " << msg.to_string() << endl;
    return out;
}

struct Graph {
    vector<Node> nodes;
    zmq::context_t zmq_ctx;

    Graph(fstream &f, int n) {
        zmq_ctx = zmq::context_t(1);
        // nodes.reserve(n);
        for(int i = 0; i < n; i++) {
            nodes.push_back(Node(n));
        }

        cout << "Graph constructor" << endl;
        char * line = new char[100];
        while(f.getline(line, 100)) {
            string s(line);
            // cout << s << endl;
            if(s == "") {
                continue;
            }
            stringstream ss(s);
            int i;
            ss >> i;
            int j;
            while(ss >> j) {
                // string addr = get_addr(j - 1);
                // string self = get_addr(i - 1);
                // nodes[i - 1].sub.connect(addr);
                // cout << self << ", " << addr << endl;

                // cout << i << ", " << j << endl;
                if (i < j) {
                    auto sock_pair = get_pair(&zmq_ctx, i - 1, j - 1);

                    nodes[i - 1].socks.push_back(sock_pair.first);
                    nodes[j - 1].socks.push_back(sock_pair.second);
                    // auto pi1 = zmq::pollitem_t();
                    nodes[i - 1].poll_items.push_back(zmq::pollitem_t{static_cast<void*>(sock_pair.first), 0, ZMQ_POLLIN | ZMQ_POLLERR, 0});
                    nodes[j - 1].poll_items.push_back(zmq::pollitem_t{static_cast<void*>(sock_pair.second), 0, ZMQ_POLLIN | ZMQ_POLLERR, 0});
                }

                // ((zmq::socket_t *)nodes[i - 1].socks.back().socket)->send(zmq::str_buffer("Hi."), zmq::send_flags::none);
                // ((zmq::socket_t *)nodes[j - 1].socks.back().socket)->send(zmq::str_buffer("Hello."), zmq::send_flags::none);
                // cout << "Sent" << endl;
                // auto msg = zmq::message_t();
                // ((zmq::socket_t *)nodes[i - 1].socks.back().socket)->recv(msg, zmq::recv_flags::none);
                // cout << msg.to_string() << endl;
                // ((zmq::socket_t *)nodes[j - 1].socks.back().socket)->recv(msg, zmq::recv_flags::none);
                // cout << msg.to_string() << endl;
                // cout << "Connected " << i << " and " << j << endl;
            }
        }
        f.close();
    }

};


/// @brief Need this function coz can't spawn a thread on a method.
/// @param node 
/// @param params 
/// @param tid 
int thread_init(Node * node, Params * params, int tid) {
    /*for(int i = 0; i < params.m; i++) {
        // this_thread::sleep_for(chrono::microseconds((int)(params.exp(gen) * 1000)));
        zmq::message_t msg;
        node->sub.set(zmq::sockopt::rcvtimeo, (int)params.exp(gen)); // TODO: Granularity

        // Recv event?
        if (node->sub.recv(msg, zmq::recv_flags::none)) {
            // Logging, rule 2.
            // cout << "Recved message: " << msg.to_string() << endl;
            // printf("%d Recved message: %s\n", tid, msg.to_string().c_str());
            int * v = (int *)msg.data();
            for(int i = 0; i < msg.size() / sizeof(int); i++) {
                node->vtime[i] = max(node->vtime[i], v[i]);
                printf("%d %d %d\n", tid, i, v[i]);
            }
            
        } else if(params.bern(gen)) { // Send event?
            // cout << "Sending message" << endl;
            printf("%d Sending message\n", tid);
            node->pub.send(zmq::buffer(node->vtime.data(), node->vtime.size() * sizeof(int)));
            // node->pub.send(zmq::str_buffer("Hello world."));
        }
        // Normie event. Only rule 1.
        node->vtime[tid]++; 
    
    }*/
    
    node->thread_fn(tid, *params);

    return 0;
}


struct Context {
    Graph * graph;
    Params * params;

    Context() {
        fstream fs;
        fs.open("inp-params.txt", ios::in);
        params = new Params(fs);
        // cout << params->n << " " << params->l << " " << params->a << " " << params->m << endl;
        graph = new Graph(fs, params->n);
    }

    void thread_spawn() {
        vector<thread> threads;
        for(int i = 0; i < params->n; i++) {
            cout << "Initing thread " << i << endl;
            threads.push_back(thread(thread_init, &graph->nodes[i], params, i));
            cout << "Inited thread " << i << endl;
            this_thread::sleep_for(chrono::milliseconds(1000));
        }

        for(auto &t : threads) {
            t.join();
        }
    }
};

int main() {
    signal(SIGABRT, handle);
    Context * ctx = new Context();
    ctx->thread_spawn();

    return 0;
}
