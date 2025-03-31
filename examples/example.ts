interface User {
    name: string
}

const greet = (user: User) => `Hello ${user.name}!`;
const world: User = {
    name: "World"
};

console.log(greet(world));