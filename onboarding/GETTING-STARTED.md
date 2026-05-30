# Getting started

This repo is both a Jekyll site (the tutorial) and a working lab (the
scripts + examples). Getting set up means three things: run the site
locally (optional), provision the lab VM, and confirm the toolchain.
The tutorial chapters themselves walk through the lab in depth; this is
the orientation.

## 1. Run the site locally (optional)

The published site builds on GitHub Pages via
`.github/workflows/pages.yml`. To preview locally you need Ruby +
Jekyll:

```bash
sudo dnf install -y ruby ruby-devel @development-tools && bundle install
```

```bash
bundle exec jekyll serve --baseurl ""
```

Open http://localhost:4000/. The `--baseurl ""` overrides the
production `/ebpf-with-aya` baseurl for local dev.

## 2. Provision the lab

Full instructions are in **Chapter 1 (Prerequisites)** and **Chapter 2
(Lab setup)** on the site. The short version, from a Fedora 44 laptop:

```bash
sudo dnf install -y @virtualization virt-install qemu-img cloud-utils podman podman-compose && sudo systemctl enable --now libvirtd && sudo usermod -aG libvirt "$USER"
```

```bash
test -f ~/.ssh/id_ed25519.pub || ssh-keygen -t ed25519 -N "" -f ~/.ssh/id_ed25519
```

```bash
cd scripts/lab && ./provision-vm.sh ebpf-target
```

> Before the first provision, open the `BASE_URL` printed by
> `provision-vm.sh` and set `BASE_IMG` to the exact current
> `Fedora-Cloud-Base-Generic-44-*.qcow2` filename. It's the one value
> the script can't guess.

## 3. Bring up the observability stack

```bash
cd examples/03-observability-stack && ./demo.sh
```

Grafana is at http://127.0.0.1:3000 (anonymous admin).

## 4. Install the Aya toolchain and build hello-world

Chapter 4 is the full walkthrough. Then:

```bash
cd examples/06-hello-world && ./demo.sh
```

## 5. Running any chapter's example

Every program chapter maps to a folder under `examples/` with the same
shape, so once you've run one you've run them all:

```bash
cd examples/09-opensnoop      # the folder named at the top of the chapter
cat README.md                 # what it does, how to drive it, verification notes
./demo.sh                     # build on host -> deploy to the VM -> run (Ctrl-C to stop)
./demo.sh build               # just build on the host, don't deploy
```

`demo.sh` is self-documenting — the comment header at the top of each
one lists its subcommands and the environment variables it honors (e.g.
`VM=` to target a differently-named guest). Each example is also a
standalone Cargo workspace, so `cargo build --release` inside it works
on its own.

**What every example assumes:** the Chapter 3 observability stack is up
(`http://127.0.0.1:3000`) and the `ebpf-target` guest (Chapter 2) is
running and reachable. The **Networking** chapters (`tcpconnlat`
onward) additionally need the `ebpf-peer` guest — each of those says so
at the top.

## 6. Working on the tutorial

- Edit chapters in `_docs/NN-*.md`. Front matter: `title`, `order`,
  `part`, `description`, `duration`. The homepage and prev/next nav
  fill in automatically from `order`.
- Add runnable code under `examples/NN-name/` with a `README.md` and a
  `demo.sh`.
- Update `_plans/reconciliation-plan.md` — new claims default to
  `unverified`; promote only after a real Fedora 44 run.
- Commit per meaningful unit (Conventional Commits — see
  `CONTRIBUTING.md`), push, `gh run watch` to confirm the deploy.

## Common issues

- **`bundle install` fails on native extensions** → install
  `ruby-devel` and `@development-tools`.
- **Pages 404 in production** → `baseurl` mismatch; it must equal
  `/ebpf-with-aya`.
- **`examples/` shows up on the live site** → check `_config.yml`
  `exclude:` has `examples/` with the trailing slash.
- **`vm-ip.sh` says no lease** → cloud-init is still booting; wait and
  retry.
- **`cargo build` fails in `examples/06-hello-world`** → expected on
  first contact; compare against a fresh `cargo generate` from
  `aya-template` and reconcile. See Chapter 6's "When the build doesn't
  compile".
