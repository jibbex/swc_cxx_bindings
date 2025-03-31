interface User {
    name: string
}

const greet = (user: User) => `Hello ${user.name}!`;