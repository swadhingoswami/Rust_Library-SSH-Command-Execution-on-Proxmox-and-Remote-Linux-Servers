use std::env;
use tokio::process::Command;
use std::io::{self, Write};
use rpassword::read_password;
use tokio::time::{timeout, Duration};

/// Default Windows SSH executable path
pub const DEFAULT_WINDOWS_SSH_EXE: &str = "C:\\Windows\\System32\\OpenSSH\\ssh.exe";

/// Securely prompt for password input (hidden terminal input)
/// 
/// # Arguments
/// * `user` - SSH username
/// * `host` - SSH hostname or IP
///
/// # Returns
/// * Password entered by the user as a `String`
async fn prompt_password(user: &str, host: &str) -> String {
    print!("🔐 Enter SSH password for {}@{}: ", user, host);
    io::stdout().flush().unwrap(); // Ensure prompt prints before input
    read_password().expect("Failed to read password")
}

/// Builds an SSH command based on OS and authentication method
/// 
/// # Arguments
/// * `user` - SSH username
/// * `host` - SSH hostname or IP
/// * `command` - Command to execute remotely
/// * `password` - Password for SSH (can be empty for password-less auth)
/// * `ssh_exe` - Optional Windows-specific SSH executable path
///
/// # Returns
/// * A configured `Command` ready to be executed asynchronously
pub fn build_ssh_command(
    user: &str,
    host: &str,
    command: &str,
    password: &str,
    ssh_exe: Option<&str>,
) -> Command {
    let is_windows = env::consts::OS == "windows";

    if is_windows {
        let exe_path = ssh_exe.unwrap_or(DEFAULT_WINDOWS_SSH_EXE);
        let mut cmd = Command::new(exe_path);
        cmd.args([
            "-o", "StrictHostKeyChecking=no",
            &format!("{}@{}", user, host),
            command,
        ]);
        cmd
    } else {
        if password.is_empty() {
            // Password-less SSH (e.g. using SSH keys)
            let ssh_cmd = format!(
                "ssh -o StrictHostKeyChecking=no {}@{} '{}'",
                user, host, command
            );
            let mut cmd = Command::new("sh");
            cmd.arg("-c").arg(ssh_cmd);
            cmd
        } else {
            // Password-based SSH using sshpass
            let ssh_cmd = format!(
                "sshpass -p '{}' ssh -o StrictHostKeyChecking=no {}@{} '{}'",
                password, user, host, command
            );
            let mut cmd = Command::new("sh");
            cmd.arg("-c").arg(ssh_cmd);
            cmd
        }
    }
}

/// Executes an SSH command asynchronously with optional timeout
///
/// # Arguments
/// * `user` - SSH username
/// * `host` - SSH hostname or IP
/// * `password` - Optional password. If `None`, prompts interactively
/// * `command` - Command to run
/// * `ssh_exe` - Optional Windows SSH path
/// * `timeout_secs` - Optional timeout in seconds for SSH execution
pub async fn execute_ssh_command_async(
    user: &str,
    host: &str,
    password: Option<String>,
    command: &str,
    ssh_exe: Option<&str>,
    timeout_secs: Option<u64>,
) {
    let pwd = match password {
        Some(p) => p,
        None => prompt_password(user, host).await,
    };

    let mut ssh_cmd = build_ssh_command(user, host, command, &pwd, ssh_exe);

    let exec_future = ssh_cmd.output();

    let result = match timeout_secs {
        Some(secs) => timeout(Duration::from_secs(secs), exec_future).await,
        None => Ok(exec_future.await),
    };

    match result {
        Ok(Ok(output)) => {
            if output.status.success() {
                let stdout = String::from_utf8_lossy(&output.stdout);
                println!("✅ SSH Output:\n{}", stdout);
            } else {
                let stderr = String::from_utf8_lossy(&output.stderr);
                eprintln!("❌ SSH Error:\n{}", stderr);
            }
        }
        Ok(Err(err)) => {
            eprintln!("❌ Failed to run SSH command: {}", err);
        }
        Err(_) => {
            eprintln!("⏱️ SSH command timed out");
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn test_proxmox_server_command_exec() {
        let user = "root";
        let host = "10.60.25.197";
        let password = Some("Arcserve@123".to_string());
        let command = "uname -a";
        let ssh_exe = Some("C:\\Windows\\System32\\OpenSSH\\ssh.exe");

        println!("Connecting to Proxmox server...");
        execute_ssh_command_async(user, host, password, command, ssh_exe, Some(10)).await;
    }

    /* 
    #[tokio::test]
    async fn test_passwordless_ssh_linux() {
        let user = "ubuntu";
        let host = "192.168.1.100";
        let password = Some("".to_string()); // Password-less SSH
        let command = "whoami";

        println!("Testing passwordless SSH...");
        execute_ssh_command_async(user, host, password, command, None, Some(5)).await;
    }
    */
}
