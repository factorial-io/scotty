import config from "@factorial/docs/config.js";

export default function (defaultConfig) {
    return config(defaultConfig, {
        projectName: "Scotty",
        input: "./content",
        output: "./_site",
        githubUrl: "https://github.com/factorial-io/scotty",
        openSource: true,
        heroImage: {
            src: "/assets/hero.png",
            width: 720,
            height: 600,
        },
        logo: {
            src: "/assets/logo.svg",
            width: 115,
            height: 30,
        },
        footerLogo: {
            src: "/assets/logo-white.svg",
            width: 115,
            height: 30,
        },
        menu: [
            "guide",
            "first-steps",
            "architecture",
            "installation",
            "configuration",
            "observability",
            "cli",
            "oauth-authentication",
            "authorization",
            "changelog",
        ],
    });
}
