#include "structs.cpp"

int main() {
    init = chrono::system_clock::now();
    Context<SKNode> * ctx = new Context<SKNode>("inp-params.txt", "sk-out.txt");
    ctx->thread_spawn();
    return 0;
}