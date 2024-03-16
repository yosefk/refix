#include <assert.h>

void f(void* p) {
    assert(p);
    assert(0 && "we are supposed to fail here");
}

extern char buf[];

int main() {
    /* our source code */
    f(buf);
}
