#!/bin/bash
set -euo pipefail  # Exit on error, undefined vars, and pipeline failures
IFS=$'\n\t'       # Stricter word splitting

has_cmd() {
    command -v "$1" >/dev/null 2>&1
}

ipv6_enabled() {
    [ -s /proc/net/if_inet6 ]
}

validate_domain() {
    [[ "$1" =~ ^[a-zA-Z0-9][a-zA-Z0-9.-]+\.[a-zA-Z]{2,}$ ]]
}

IP6TABLES_AVAILABLE=0
if has_cmd ip6tables; then
    IP6TABLES_AVAILABLE=1
fi

if ipv6_enabled && [ "$IP6TABLES_AVAILABLE" -ne 1 ]; then
    if has_cmd sysctl; then
        if sysctl -w net.ipv6.conf.all.disable_ipv6=1 >/dev/null 2>&1 \
            && sysctl -w net.ipv6.conf.default.disable_ipv6=1 >/dev/null 2>&1; then
            echo "IPv6 disabled because ip6tables is not available"
        else
            echo "ERROR: IPv6 is enabled but ip6tables is missing"
            exit 1
        fi
    else
        echo "ERROR: IPv6 is enabled but ip6tables is missing"
        exit 1
    fi
fi

ENABLE_IPV6=0
if ipv6_enabled && [ "$IP6TABLES_AVAILABLE" -eq 1 ]; then
    ENABLE_IPV6=1
fi

# Read allowed domains from file
ALLOWED_DOMAINS_FILE="/etc/beacon-code/allowed_domains.txt"
ALLOWED_DOMAINS=()
if [ -n "${OPENAI_ALLOWED_DOMAINS:-}" ]; then
    env_domains=()
    IFS=$' \t\n' read -r -a env_domains <<< "$OPENAI_ALLOWED_DOMAINS"
    for domain in "${env_domains[@]}"; do
        if [ -z "$domain" ]; then
            continue
        fi
        if ! validate_domain "$domain"; then
            echo "ERROR: Invalid domain format: $domain"
            exit 1
        fi
        ALLOWED_DOMAINS+=("$domain")
    done
    if [ "${#ALLOWED_DOMAINS[@]}" -eq 0 ]; then
        echo "ERROR: OPENAI_ALLOWED_DOMAINS is empty."
        exit 1
    fi
    mkdir -p "$(dirname "$ALLOWED_DOMAINS_FILE")"
    printf '%s\n' "${ALLOWED_DOMAINS[@]}" > "$ALLOWED_DOMAINS_FILE"
    chmod 444 "$ALLOWED_DOMAINS_FILE"
    chown root:root "$ALLOWED_DOMAINS_FILE"
    echo "Using domains from OPENAI_ALLOWED_DOMAINS: ${ALLOWED_DOMAINS[*]}"
elif [ -f "$ALLOWED_DOMAINS_FILE" ]; then
    while IFS= read -r domain; do
        if [ -z "$domain" ]; then
            continue
        fi
        if ! validate_domain "$domain"; then
            echo "ERROR: Invalid domain format in $ALLOWED_DOMAINS_FILE: $domain"
            exit 1
        fi
        ALLOWED_DOMAINS+=("$domain")
    done < "$ALLOWED_DOMAINS_FILE"
    if [ "${#ALLOWED_DOMAINS[@]}" -eq 0 ]; then
        echo "ERROR: No allowed domains specified"
        exit 1
    fi
    echo "Using domains from file: ${ALLOWED_DOMAINS[*]}"
else
    # Fallback to default domains
    ALLOWED_DOMAINS=("api.openai.com")
    echo "Domains file not found, using default: ${ALLOWED_DOMAINS[*]}"
fi

# Ensure we have at least one domain
if [ ${#ALLOWED_DOMAINS[@]} -eq 0 ]; then
    echo "ERROR: No allowed domains specified"
    exit 1
fi

# Flush existing rules and delete existing ipsets
iptables -F
iptables -X
iptables -t nat -F
iptables -t nat -X
iptables -t mangle -F
iptables -t mangle -X
if [ "$ENABLE_IPV6" -eq 1 ]; then
    ip6tables -F
    ip6tables -X
    ip6tables -t nat -F
    ip6tables -t nat -X
    ip6tables -t mangle -F
    ip6tables -t mangle -X
fi
ipset destroy allowed-domains 2>/dev/null || true
ipset destroy allowed-domains6 2>/dev/null || true

# First allow DNS and localhost before any restrictions
# Allow outbound DNS
iptables -A OUTPUT -p udp --dport 53 -j ACCEPT
# Allow inbound DNS responses
iptables -A INPUT -p udp --sport 53 -j ACCEPT
# Allow localhost
iptables -A INPUT -i lo -j ACCEPT
iptables -A OUTPUT -o lo -j ACCEPT
if [ "$ENABLE_IPV6" -eq 1 ]; then
    ip6tables -A OUTPUT -p udp --dport 53 -j ACCEPT
    ip6tables -A INPUT -p udp --sport 53 -j ACCEPT
    ip6tables -A INPUT -i lo -j ACCEPT
    ip6tables -A OUTPUT -o lo -j ACCEPT
    ip6tables -A OUTPUT -p icmpv6 -j ACCEPT
    ip6tables -A INPUT -p icmpv6 -j ACCEPT
fi

# Create ipset with CIDR support
ipset create allowed-domains hash:net
if [ "$ENABLE_IPV6" -eq 1 ]; then
    ipset create allowed-domains6 hash:net family inet6
fi

# Resolve and add other allowed domains
for domain in "${ALLOWED_DOMAINS[@]}"; do
    echo "Resolving $domain..."
    ips_v4=$(dig +short A "$domain")
    ips_v6=$(dig +short AAAA "$domain")
    ipv4_ips=()
    ipv6_ips=()

    if [ -n "$ips_v4" ]; then
        while read -r ip; do
            if [ -z "$ip" ]; then
                continue
            fi
            if [[ "$ip" =~ ^[0-9]{1,3}\.[0-9]{1,3}\.[0-9]{1,3}\.[0-9]{1,3}$ ]]; then
                ipv4_ips+=("$ip")
            else
                echo "Skipping non-IPv4 answer for $domain: $ip"
            fi
        done < <(echo "$ips_v4")
    fi

    if [ -n "$ips_v6" ]; then
        while read -r ip; do
            if [ -z "$ip" ]; then
                continue
            fi
            if [[ "$ip" =~ ^[0-9a-fA-F:]+$ ]]; then
                ipv6_ips+=("$ip")
            else
                echo "Skipping non-IPv6 answer for $domain: $ip"
            fi
        done < <(echo "$ips_v6")
    fi

    if [ "${#ipv4_ips[@]}" -eq 0 ] && { [ "$ENABLE_IPV6" -ne 1 ] || [ "${#ipv6_ips[@]}" -eq 0 ]; }; then
        echo "ERROR: Failed to resolve $domain"
        exit 1
    fi

    for ip in "${ipv4_ips[@]}"; do
        echo "Adding $ip for $domain (IPv4)"
        ipset add allowed-domains "$ip"
    done

    if [ "$ENABLE_IPV6" -eq 1 ]; then
        for ip in "${ipv6_ips[@]}"; do
            echo "Adding $ip for $domain (IPv6)"
            ipset add allowed-domains6 "$ip"
        done
    fi
done

# Get host IP from default route
HOST_IP=$(ip route | grep default | cut -d" " -f3)
if [ -z "$HOST_IP" ]; then
    echo "ERROR: Failed to detect host IP"
    exit 1
fi

HOST_NETWORK=$(echo "$HOST_IP" | sed "s/\.[0-9]*$/.0\/24/")
echo "Host network detected as: $HOST_NETWORK"

# Set up remaining iptables rules
iptables -A INPUT -s "$HOST_NETWORK" -j ACCEPT
iptables -A OUTPUT -d "$HOST_NETWORK" -j ACCEPT

# Set default policies to DROP first
iptables -P INPUT DROP
iptables -P FORWARD DROP
iptables -P OUTPUT DROP
if [ "$ENABLE_IPV6" -eq 1 ]; then
    ip6tables -P INPUT DROP
    ip6tables -P FORWARD DROP
    ip6tables -P OUTPUT DROP
fi

# First allow established connections for already approved traffic
iptables -A INPUT -m state --state ESTABLISHED,RELATED -j ACCEPT
iptables -A OUTPUT -m state --state ESTABLISHED,RELATED -j ACCEPT
if [ "$ENABLE_IPV6" -eq 1 ]; then
    ip6tables -A INPUT -m state --state ESTABLISHED,RELATED -j ACCEPT
    ip6tables -A OUTPUT -m state --state ESTABLISHED,RELATED -j ACCEPT
fi

# Then allow only specific outbound traffic to allowed domains
iptables -A OUTPUT -m set --match-set allowed-domains dst -j ACCEPT
if [ "$ENABLE_IPV6" -eq 1 ]; then
    ip6tables -A OUTPUT -m set --match-set allowed-domains6 dst -j ACCEPT
fi

# Append final REJECT rules for immediate error responses
# For TCP traffic, send a TCP reset; for UDP, send ICMP port unreachable.
iptables -A INPUT -p tcp -j REJECT --reject-with tcp-reset
iptables -A INPUT -p udp -j REJECT --reject-with icmp-port-unreachable
iptables -A OUTPUT -p tcp -j REJECT --reject-with tcp-reset
iptables -A OUTPUT -p udp -j REJECT --reject-with icmp-port-unreachable
iptables -A FORWARD -p tcp -j REJECT --reject-with tcp-reset
iptables -A FORWARD -p udp -j REJECT --reject-with icmp-port-unreachable
if [ "$ENABLE_IPV6" -eq 1 ]; then
    ip6tables -A INPUT -p tcp -j REJECT --reject-with tcp-reset
    ip6tables -A INPUT -p udp -j REJECT --reject-with icmp6-port-unreachable
    ip6tables -A OUTPUT -p tcp -j REJECT --reject-with tcp-reset
    ip6tables -A OUTPUT -p udp -j REJECT --reject-with icmp6-port-unreachable
    ip6tables -A FORWARD -p tcp -j REJECT --reject-with tcp-reset
    ip6tables -A FORWARD -p udp -j REJECT --reject-with icmp6-port-unreachable
fi

echo "Firewall configuration complete"
echo "Verifying firewall rules..."
if curl --connect-timeout 5 https://example.com >/dev/null 2>&1; then
    echo "ERROR: Firewall verification failed - was able to reach https://example.com"
    exit 1
else
    echo "Firewall verification passed - unable to reach https://example.com as expected"
fi

# Always verify OpenAI API access is working
if ! curl --connect-timeout 5 https://api.openai.com >/dev/null 2>&1; then
    echo "ERROR: Firewall verification failed - unable to reach https://api.openai.com"
    exit 1
else
    echo "Firewall verification passed - able to reach https://api.openai.com as expected"
fi

MARKER_PATH="/etc/beacon-code/firewall.active"
mkdir -p "$(dirname "$MARKER_PATH")"
date -u +"%Y-%m-%dT%H:%M:%SZ" > "$MARKER_PATH"
chmod 444 "$MARKER_PATH" || true

READY_FILE="/etc/beacon-code/firewall.ready"
mkdir -p "$(dirname "$READY_FILE")"
touch "$READY_FILE"
chmod 444 "$READY_FILE"
echo "Firewall readiness marker created at ${READY_FILE}"
