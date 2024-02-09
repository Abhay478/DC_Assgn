#include <iostream>

#include <zmq.hpp>

int main() {
    zmq::context_t ctx(1);
    zmq::socket_t sock(ctx, ZMQ_ROUTER);
    sock.bind("inproc://sock5555");
    zmq::socket_t othersock(ctx, ZMQ_ROUTER);
    othersock.connect("inproc://sock5555");
    std::cout << othersock.send(zmq::str_buffer("Hello world.")).value() << std::endl;

    zmq::message_t msg;
    sock.recv(msg);
    std::cout << "Received: " << msg.to_string() << std::endl;
    return 0;
}