#include "structs.cpp"

int main() {
    // signal(SIGABRT, handle);
    // signal(SIGSEGV, handle);
    init = chrono::system_clock::now();
    Context<Node> * ctx = new Context<Node>("inp-params.txt", "vc-out.txt");
    ctx->thread_spawn();
    return 0;
}