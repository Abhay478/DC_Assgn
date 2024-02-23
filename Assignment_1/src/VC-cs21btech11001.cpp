#include "structs.cpp"

int main(int argc, char *argv[]) {
    // signal(SIGABRT, handle);
    if(argc > 1) ip = argv[1];
    else ip = "127.0.0.1";
    // signal(SIGINT, handle);
    Context<Node> * ctx = new Context<Node>("inp-params.txt", "out/vc-out.txt");
    init = chrono::system_clock::now();
    ctx->thread_spawn();
    return 0;
}