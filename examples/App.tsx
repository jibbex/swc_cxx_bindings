import * as React from 'react';

export const App = () => {
	const [click, setClick] = React.useState(0);
	
	const handleClick: () => void = () => setClick((num: number) => num++);

	return (
		<div>
			<h1>{click}</h1>
			<button onClick={handleClick}>increase</button>
		</div>
	);
};  