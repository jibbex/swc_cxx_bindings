#include "../swc.h"
#include <iostream>

int main(int argc, char const *argv[]) {
    constexpr auto RESET = "\033[0m";
    constexpr auto YELLOW = "\033[33m";
    constexpr auto RED = "\033[31m";
    constexpr auto UNDERLINE = "\033[4m";

    if (argc < 2) {
        std::cerr << UNDERLINE << "Usage"
            << RESET << ": " << YELLOW
            << argv[0] << " <file_name>"
            << RESET << std::endl;

        return 1;
    }

    const char* ts_code = R"(
interface User {
    name: string
}

const greet = (user: User) => `Hello ${user.name}!`;
const world: User = {
    name: "World"
};

console.log(greet(world));
    )";

    const char* file_name = "test.ts";
    char *error;
    auto code = swc::transpile(file_name, ts_code);
    auto js_code = swc::minify_js(swc::compile_file(argv[1], error), error);

    std::cout << "\n" << YELLOW << UNDERLINE << "Input:\n" << RESET << ts_code << std::endl;
    std::cout << "\n" << YELLOW << UNDERLINE << "Output:\n\n"<< RESET << code << std::endl;

    if (error != nullptr) {
        std::cerr << "\n" << RED << UNDERLINE << "Error:\n\n" << RESET << error << std::endl;
        swc::free_string(error);
    } else {
        std::cout << "\n" << YELLOW << UNDERLINE << "Compiled from File and Minified:\n\n" << RESET << js_code << std::endl;
    }

    swc::free_string(code);
    swc::free_string(js_code);

    return 0;
}