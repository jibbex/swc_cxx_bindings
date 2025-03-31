#include "../swc.h"
#include <iostream>

int main(int argc, char const *argv[]) {
    constexpr auto RESET = "\033[0m";
    constexpr auto YELLOW = "\033[33m";
    constexpr auto UNDERLINE = "\033[4m";

    if (argc < 2) {
        std::cerr << UNDERLINE
            << YELLOW << "Usage: "
            << RESET << YELLOW
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

    const char* file_name = argv[1];
    auto code = swc::transpile(file_name, ts_code);
    auto code_from_file = swc::transpile_file(file_name);

    std::cout << "\n" << YELLOW << UNDERLINE << "Input:\n" << RESET << ts_code << std::endl;
    std::cout << "\n" << YELLOW << UNDERLINE << "Output:\n\n"<< RESET << code << std::endl;
    std::cout << "\n" << YELLOW << UNDERLINE << "File:\n\n"<< RESET << code_from_file << std::endl;

    swc::free_string(code);

    return 0;
}