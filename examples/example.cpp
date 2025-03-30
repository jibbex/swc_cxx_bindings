#include "../swc_ts.h"
#include <iostream>

int main() {
    const char* ts_code = R"(
        interface User { name: string }
        const greet = (user: User) => `Hello ${user.name}!`;
    )";

    const char* file_name = "example.ts";
    Output transpiled = transpile_js(ts_code, file_name);

    std::cout << transpiled.code << '\n'
        << transpiled.map << '\n'
        << transpiled.output << '\n'
        << transpiled.diagnostics << std::endl;

    free_string(transpiled.code);
    free_string(transpiled.map);
    free_string(transpiled.output);
    free_string(transpiled.diagnostics);

    return 0;
}