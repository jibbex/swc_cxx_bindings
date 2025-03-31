#include "../swc.h"
#include <iostream>

int main() {
    const char* ts_code = R"(
        interface User { name: string }
        const greet = (user: User) => `Hello ${user.name}!`;
    )";

    const char* file_name = "example.ts";
    auto transpiled = swc::transpile(ts_code, file_name);

    std::cout << transpiled << std::endl;

    swc::free_string(transpiled);

    return 0;
}