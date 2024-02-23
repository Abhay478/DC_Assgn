#include "structs.cpp"

int main(int argc, char *argv[]) {
    if(argc > 1) ip = argv[1];
    else ip = "127.0.0.1";
    // signal(SIGINT, handle);
    Context<SKNode> * ctx = new Context<SKNode>("inp-params.txt", "out/sk-out.txt");
    init = chrono::system_clock::now();
    ctx->thread_spawn();
    return 0;
}