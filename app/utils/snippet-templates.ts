interface SnippetTemplate {
  name: string;
  command: string;
  description: string;
  group: string;
  tags: string[];
}

/** Pre-built snippets for common server administration tasks. */
export const snippetTemplates: SnippetTemplate[] = [
  {
    name: 'Disk usage',
    command: 'df -h',
    description: 'Show disk space usage for all mounted filesystems',
    group: 'System',
    tags: ['disk', 'monitoring'],
  },
  {
    name: 'Memory usage',
    command: 'free -h',
    description: 'Show memory and swap usage',
    group: 'System',
    tags: ['memory', 'monitoring'],
  },
  {
    name: 'Top processes by CPU',
    command: 'ps aux --sort=-%cpu | head -n 15',
    description: 'List the busiest processes',
    group: 'System',
    tags: ['process', 'monitoring'],
  },
  {
    name: 'System load & uptime',
    command: 'uptime',
    description: 'Show uptime and load averages',
    group: 'System',
    tags: ['monitoring'],
  },
  {
    name: 'OS release info',
    command: 'cat /etc/os-release',
    description: 'Show distribution details',
    group: 'System',
    tags: ['info'],
  },
  {
    name: 'Tail syslog',
    command: 'tail -n 50 -f /var/log/syslog',
    description: 'Follow the system log (journalctl on systemd distros)',
    group: 'Logs',
    tags: ['logs'],
  },
  {
    name: 'Recent auth attempts',
    command: 'grep -E "Failed password|Accepted password" /var/log/auth.log | tail -n 20',
    description: 'Show recent SSH login activity',
    group: 'Logs',
    tags: ['logs', 'security'],
  },
  {
    name: 'Journal errors (last hour)',
    command: 'journalctl -p err --since "1 hour ago" --no-pager',
    description: 'Show error-level journal entries from the last hour',
    group: 'Logs',
    tags: ['logs', 'systemd'],
  },
  {
    name: 'Listening ports',
    command: 'ss -tulnp',
    description: 'List all listening TCP/UDP sockets',
    group: 'Network',
    tags: ['network', 'ports'],
  },
  {
    name: 'Ping a host',
    command: 'ping -c 4 {{target}}',
    description: 'Send 4 ICMP echo requests to a target host',
    group: 'Network',
    tags: ['network'],
  },
  {
    name: 'DNS lookup',
    command: 'dig +short {{domain}}',
    description: 'Resolve DNS records for a domain',
    group: 'Network',
    tags: ['network', 'dns'],
  },
  {
    name: 'Update package lists & upgrade',
    command: 'sudo apt update && sudo apt upgrade -y',
    description: 'Debian/Ubuntu full upgrade',
    group: 'Packages',
    tags: ['apt', 'upgrade'],
  },
  {
    name: 'List Docker containers',
    command: 'docker ps -a',
    description: 'Show all Docker containers',
    group: 'Docker',
    tags: ['docker'],
  },
  {
    name: 'Docker container logs',
    command: 'docker logs --tail 100 -f {{container}}',
    description: 'Follow the last 100 log lines of a container',
    group: 'Docker',
    tags: ['docker', 'logs'],
  },
  {
    name: 'Docker disk usage',
    command: 'docker system df',
    description: 'Show Docker disk usage',
    group: 'Docker',
    tags: ['docker', 'disk'],
  },
  {
    name: 'Systemd service status',
    command: 'systemctl status {{service}}',
    description: 'Show the status of a systemd service',
    group: 'Services',
    tags: ['systemd'],
  },
  {
    name: 'Restart systemd service',
    command: 'sudo systemctl restart {{service}}',
    description: 'Restart a systemd service',
    group: 'Services',
    tags: ['systemd'],
  },
  {
    name: 'Nginx config test',
    command: 'sudo nginx -t',
    description: 'Validate the nginx configuration',
    group: 'Services',
    tags: ['nginx'],
  },
  {
    name: 'Find large files',
    command: 'find {{path}} -type f -size +100M -exec ls -lh {} \\;',
    description: 'Find files larger than 100MB under a path',
    group: 'Files',
    tags: ['files', 'disk'],
  },
  {
    name: 'Directory size summary',
    command: 'du -h --max-depth=1 {{path}} | sort -hr | head -n 20',
    description: 'Largest subdirectories of a path',
    group: 'Files',
    tags: ['files', 'disk'],
  },
];
