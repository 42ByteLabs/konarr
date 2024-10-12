<!-- markdownlint-disable -->
<div align="center">
<h1>Konarr</h1>

[![GitHub](https://img.shields.io/badge/github-%23121011.svg?style=for-the-badge&logo=github&logoColor=white)][github]
[![Crates.io Version](https://img.shields.io/crates/v/konarr?style=for-the-badge)][crates-io]
[![Crates.io Downloads (recent)](https://img.shields.io/crates/dr/konarr?style=for-the-badge)][crates-io]
[![GitHub Stars](https://img.shields.io/github/stars/42ByteLabs/konarr?style=for-the-badge)][github]
[![GitHub Issues](https://img.shields.io/github/issues/42ByteLabs/konarr?style=for-the-badge)][github-issues]
[![Licence](https://img.shields.io/github/license/42ByteLabs/konarr?style=for-the-badge)][license]

⚠️  This is currently a work in progress and still in the early stages of development ⚠️

</div>
<!-- markdownlint-restore -->

## Overview

[Konarr][konarr] is a simple, easy-to-use web interface for monitoring your servers, clusters, and containers for supply chain attacks.
It is designed to be lightweight and fast, with minimal resource usage. 

It is written in [Rust][rust-lang], uses [Rocker][rocket] for the web server, and [Vue.js](https://vuejs.org/) for the front-end.

<details>
<summary><strong>Origin Story</strong></summary>

This project came out of the need to monitor my homelab for insecure dependencies / components.
All the products that offer this are proprietary and cost money to use.

[In December 2021, Log4Shell (CVE-2021-44228)](https://en.wikipedia.org/wiki/Log4Shell) came dropped and like most of the world I was running around trying to find if I had a service using it.
Turned out I was but it was a painful process in finding if I was even using it.

**Name Origin:**

Konarr is from the name [Konar quo Maten](https://oldschool.runescape.wiki/w/Konar_quo_Maten) (translated as Konar the Hunter) from the game [Old School Runescape](https://oldschool.runescape.com/).

</details>

## ✨ Features

- Simple, easy-to-use web interface
- Blazing fast performance with minimal resource usage (written in [Rust][rust-lang] 🦀)
- Real-time monitoring of your containers
- Software Bill of Materials (SBOM) for your containers
- Supply chain attack monitoring

## 📚 Documentation

TODO: Update this section with the correct information.

## 🚀 Quick Start

### Server

The Konarr Server is a Rust (Server) and VueJS (frontend) that 

#### Konarr Server using Docker Compose

```bash
# Download docker-compose config
curl https://raw.githubusercontent.com/42ByteLabs/konarr/refs/heads/main/docker-compose.yml

# Spin up container using Docker Compose
docker-compose up -d
```

*Note:* Podman-compose also works.

#### Konarr Server using Docker

```bash
docker run -it --rm \
    -p 9000:9000 \
    -v ./data:/data -v ./config:/config \
    ghcr.io/42bytelabs/konarr:latest
```

### Agent

#### Running Agent in Docker

```bash
docker run -it --rm \
    -e KONARR_INSTANCE \
    -e KONARR_AGENT_TOKEN \
    -e KONARR_PROJECT_ID \
    ghcr.io/42bytelabs/konarr-agent:latest
```

## ❤️  Maintainers / Contributors

<!-- ALL-CONTRIBUTORS-LIST:START - Do not remove or modify this section -->
<!-- prettier-ignore-start -->
<!-- markdownlint-disable -->
<table>
  <tbody>
    <tr>
      <td align="center" valign="top" width="14.28%"><a href="https://geekmasher.dev"><img src="https://avatars.githubusercontent.com/u/2772944?v=4?s=100" width="100px;" alt="Mathew Payne"/><br /><sub><b>Mathew Payne</b></sub></a><br /><a href="#code-GeekMasher" title="Code">💻</a> <a href="#review-GeekMasher" title="Reviewed Pull Requests">👀</a></td>
    </tr>
  </tbody>
</table>

<!-- markdownlint-restore -->
<!-- prettier-ignore-end -->

<!-- ALL-CONTRIBUTORS-LIST:END -->

## 🦸 Support

Please create [GitHub Issues][github-issues] if there are bugs or feature requests.

This project uses [Semantic Versioning (v2)][semver] and with major releases, breaking changes will occur.

## 📓 License

This project is licensed under the terms of the Apache2 open source license.
Please refer to [Apache2][license] for the full terms.

<!-- Resources -->

[license]: ./LICENSE
[crates-io]: https://crates.io/crates/konarr
[docs]: https://docs.rs/konarr/latest/konarr
[semver]: https://semver.org/
[rust-lang]: https://www.rust-lang.org/
[rocket]: https://rocket.rs/

[konarr]: https://github.com/42ByteLabs/konarr
[github]: https://github.com/42ByteLabs/konarr
[github-issues]: https://github.com/42ByteLabs/konarr/issues


