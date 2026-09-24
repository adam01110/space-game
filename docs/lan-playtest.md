# LAN playtest

The browser client needs HTTPS, and game traffic is UDP. `just web` is loopback
HTTP only and does not work from another device. No persistent NixOS firewall
change is needed.

Find this machine's IPv4 address on the network shared with the testers
(`ip -4 addr`) and use it as `HOST_IP`.

1. Start the server and the browser host in separate terminals:

   ```sh
   just server-lan HOST_IP
   just web-lan HOST_IP
   ```

   `web-lan` builds the production bundle when `web/dist` is missing. Answer
   **yes** to rebuild after changing client code. The server prints a new
   WebTransport certificate digest on each start; the guest endpoint forwards
   the matching digest and short-lived connection token automatically.

2. `web-lan` prints the path to Caddy's public root CA (`root.crt`). Wait for
   Caddy to start, then transfer that file only to testers over a trusted
   channel. Testers verify its fingerprint with you out of band and install it
   as a trusted root CA. Remove it after testing. Never share Caddy's private
   key. Bypassing a certificate warning is not a substitute.

3. For each tester, run a terminal with their IPv4 address (not `HOST_IP`):

   ```sh
   just playtest-firewall TESTER_IP
   ```

   The recipe prompts for sudo, then allows only that source IP to reach TCP
   8000 and UDP 5000 on its directly connected network interface. Ctrl-C removes
   the rules. Testers open `https://HOST_IP:8000/` and press Play. The guest
   endpoint stays on loopback behind Caddy's `/connect` proxy. Testers must also
   be able to reach each other over the LAN; some guest Wi-Fi networks isolate
   clients even when they share an address range.

The rules disappear on firewall reload or reboot. If the script is killed
without a cleanup signal, inspect
`sudo nft -a list chain inet nixos-fw input-allow` for rules marked
`space-game-playtest-*` and delete their handles with
`sudo nft delete rule inet nixos-fw input-allow handle HANDLE`.
