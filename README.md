# rss_autogen_giscus

[Giscus](https://github.com/giscus/giscus) offers a great solution to bring commenting support to your website, but it comes with the caveat that authorizing the app grants permissions to all repos. You can still comment on GitHub itself, but the discussion post itself is only created when an authenticated user leaves one first. This program solves that, running as a container that can check for new blog posts from an RSS feed, and generating a compatible discussion for Giscus.

## Usage

_Note: currently, there are some issues with running the container image. For now, it's recommended to locally build the program, and then load it into automation from there. The container image linked in the GitHub Action is **not** available yet._

1. Enable [Giscus](https://github.com/giscus/giscus) in your repo. When choosing the page to discussions mapping, select **"Discussion title contains page pathname"**.
2. Clone the [GitHub Action](.github/actions/rss_autogen_giscus/action.yaml) from this repo into your own. Replace the `website_rss_url` and `discussion_category` with your own inputs, and optionally provide `lookback_days`.
3. Copy the [workflow job](.github/workflows/generate_comments.yaml.template) from this repo. You can modify the trigger as needed. Take note of `lookback_days`, as it may recreate an existing post if the program is unintentionally triggered.

## Contributing

Feel free to open an issue or PR in this repo.

## License

Both the program and the image are licensed under [Apache 2.0](LICENSE).