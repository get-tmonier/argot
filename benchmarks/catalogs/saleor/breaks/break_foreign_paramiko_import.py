# Break: paramiko (imported in the hunk) opens an SSH/SFTP session to ship an export, replacing django storages
"""Break fixture — not for import."""

import logging

logger = logging.getLogger(__name__)


# Decoy — idiomatic saleor-style helper, NOT inside the hunk range
def export_file_name(export_id: int) -> str:
    return f"export-{export_id}.csv"


# hunk starts here
import paramiko


def upload_export_over_sftp(host: str, remote_path: str, local_file: str) -> None:
    client = paramiko.SSHClient()
    client.set_missing_host_key_policy(paramiko.AutoAddPolicy())
    client.connect(host, username="saleor", key_filename="/etc/saleor/id_rsa")
    sftp = client.open_sftp()
    sftp.put(local_file, remote_path)
    sftp.close()
    client.close()
# hunk ends here
