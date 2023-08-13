/** @type {import('tailwindcss').Config} */
module.exports = {
	content: ["./src/**/*.rs"],
	theme: {
		extend: {
			gridTemplateColumns: {
				subgrid: "subgrid",
			},
			gridTemplateRows: {
				subgrid: "subgrid",
			},
			gridAutoRows: {
				// cards: "1fr auto",
				cards: "min-content auto",
			},
		},
	},
	plugins: [require("@tailwindcss/typography"), require("daisyui")],
};
