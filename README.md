# rss_autogen_giscus

[Giscus](https://github.com/giscus/giscus) offers a great solution to bring commenting support to your website, but it comes with the caveat that authorizing the app grants permissions to all repos. You can still comment on GitHub itself, but the discussion post itself is only created when an authenticated user leaves one first. This program solves that, running as a container that can check for new blog posts from an RSS feed, and generating a compatible discussion for Giscus.

## Usage

### Binary

1. Install the binary with `cargo install rss_autogen_giscus`.
2. Get a [personal access token](https://docs.github.com/en/authentication/keeping-your-account-and-data-secure/managing-your-personal-access-tokens) with write permissions for GitHub discussions 
3. Set the environment variables as specified in [the documentation](https://docs.rs/rss_autogen_giscus/latest/rss_autogen_giscus/struct.HttpClients.html#method.init).
4. Run the program: `rss_autogen_giscus`

You can also use the provided container image:

```bash
podman pull ghcr.io/cam-rod/rss_autogen_giscus:latest
podman run --rm -it -e DISCUSSION_CATEGORY=Blogs [...] rss_autogen_giscus:latest
```

### GitHub Actions

1. Enable [Giscus](https://github.com/giscus/giscus) in your repo. When choosing the page to discussions mapping, select **"Discussion title contains page pathname"**.
2. Copy the [workflow job](.github/workflows/generate_comments.yaml.template) from this repo. Edit the environment variables, and modify the trigger as needed. Take note of `LOOKBACK_DAYS`, as it may recreate an existing post if the program is unintentionally triggered.

## Contributing

Feel free to open an issue or PR in this repo.

## License

Both the program and container image are licensed under [Apache 2.0](LICENSE).