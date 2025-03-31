#include "../swc.h"
#include <iostream>

int main() {
    const char* ts_code = R"(
        interface User { name: string }
        const greet = (user: User) => `Hello ${user.name}!`;
    )";

    const char* file_name = "example.ts";
    auto code = swc::transpile(file_name, ts_code);
//    auto code_from_file = swc::transpile_file(file_name);

    std::cout << code << std::endl;

    swc::free_string(code);

    return 0;
}