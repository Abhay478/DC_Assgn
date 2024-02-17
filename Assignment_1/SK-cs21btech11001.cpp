#include "structs.cpp"

int main() {
    signal(SIGSEGV, handle);
    init = chrono::system_clock::now();
    Context<SKNode> * ctx = new Context<SKNode>("inp-params.txt", "sk-out.txt");
    ctx->thread_spawn();
    return 0;
}