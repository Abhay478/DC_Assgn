#include "structs.cpp"

int main() {
    // signal(SIGABRT, handle);
    // signal(SIGSEGV, handle);
    Context<Node> * ctx = new Context<Node>("inp-params.txt", "out/vc-out.txt");
    init = chrono::system_clock::now();
    ctx->thread_spawn();
    return 0;
}