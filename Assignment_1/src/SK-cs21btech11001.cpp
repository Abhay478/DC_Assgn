#include "structs.cpp"

int main() {
    signal(SIGINT, handle);
    Context<SKNode> * ctx = new Context<SKNode>("inp-params.txt", "out/sk-out.txt");
    init = chrono::system_clock::now();
    ctx->thread_spawn();
    return 0;
}