# Browser playtest on the same LAN

The game needs both HTTPS for the browser page and UDP for WebTransport. The
ordinary `just web` recipe is loopback HTTP only; it does not work from another
device. This setup does not require a persistent NixOS firewall change.

1. Find this machine's IPv4 address on the network shared with the testers
   (`ip -4 addr`). Use that address as `HOST_IP` in both commands below. Run in
   separate terminals:

   ```sh
   just server-lan HOST_IP
   just web-lan HOST_IP
   ```

   `web-lan` builds the production browser client when `web/dist` does not
   exist. If asked about an existing bundle, choose **yes** after changing
   client code. The server prints a fresh WebTransport certificate digest on
   each start; the guest endpoint delivers the matching digest and short-lived
   connection token automatically.

2. Caddy creates a local HTTPS certificate authority (CA). `web-lan` prints
   the path to its public `root.crt`. Wait for Caddy to start before copying
   it. Transfer **only this public certificate** to testers through a trusted
   channel. They must verify
   its fingerprint with you out of band and install it as a trusted root CA in
   the OS/browser they use. This grants the CA authority to issue certificates
   trusted by their browser: remove it after testing. Never share Caddy's
   private key. A certificate warning or bypass is not a substitute for trust.

3. For each tester, run a separate terminal with their IPv4 address (not
   `HOST_IP`):

   ```sh
   just playtest-firewall TESTER_IP
   ```

   This prompts for sudo and allows only that source IP on its directly
   connected network interface to reach TCP 8000 and UDP 5000. Keep the
   command running during the test; Ctrl-C removes its rules. After trusting
   the CA, testers open `https://HOST_IP:8000/` and press Play. The guest
   endpoint remains bound to loopback and is reached through Caddy's `/connect`
   proxy. Testers must be able to reach each other over the LAN: some guest
   Wi-Fi networks isolate clients even when they share an address range.

The rules are transient and disappear on firewall reload/reboot. If the
firewall script is killed without a cleanup signal (for example SIGKILL),
inspect `sudo nft -a list chain inet nixos-fw input-allow` for rules marked
`space-game-playtest-*` and delete their handles with
`sudo nft delete rule inet nixos-fw input-allow handle HANDLE`.
