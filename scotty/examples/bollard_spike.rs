use anyhow::Result;
use bollard::exec::{CreateExecOptions, StartExecOptions, StartExecResults};
use bollard::query_parameters::{ListContainersOptions, LogsOptions};
use bollard::Docker;
use futures_util::stream::StreamExt;

/// Technical spike to explore bollard's container logs and exec capabilities
#[tokio::main]
async fn main() -> Result<()> {
    let docker = Docker::connect_with_local_defaults()?;

    println!("=== Bollard Technical Spike ===\n");

    // List containers
    println!("1. Available containers:");
    list_containers(&docker).await?;

    // Test logs
    println!("\n2. Testing container logs:");
    test_container_logs(&docker).await?;

    // Test exec
    println!("\n3. Testing container exec:");
    test_container_exec(&docker).await?;

    Ok(())
}

async fn list_containers(docker: &Docker) -> Result<()> {
    let containers = docker
        .list_containers(Some(ListContainersOptions {
            all: true,
            ..Default::default()
        }))
        .await?;

    for container in containers.iter().take(3) {
        if let (Some(id), Some(names), Some(image)) =
            (&container.id, &container.names, &container.image)
        {
            let default_name = "<no name>".to_string();
            let default_status = "unknown".to_string();
            let name = names.first().unwrap_or(&default_name);
            let status = container.status.as_ref().unwrap_or(&default_status);
            println!("  {} | {} | {} | {}", &id[..12], name, image, status);
        }
    }

    Ok(())
}

async fn test_container_logs(docker: &Docker) -> Result<()> {
    let container_id = find_running_container(docker).await?;
    let container_id = match container_id {
        Some(id) => id,
        None => {
            println!("No running containers found");
            return Ok(());
        }
    };

    println!("Testing logs for container: {}", &container_id[..12]);

    // Test basic logs with stdout/stderr separation
    let options = Some(LogsOptions {
        stdout: true,
        stderr: true,
        tail: "5".to_string(),
        timestamps: true,
        ..Default::default()
    });

    let mut stream = docker.logs(&container_id, options);
    let mut line_count = 0;

    while let Some(log_result) = stream.next().await {
        let log_output = log_result?;

        let (stream_type, message) = match log_output {
            bollard::container::LogOutput::StdOut { message } => ("STDOUT", message),
            bollard::container::LogOutput::StdErr { message } => ("STDERR", message),
            _ => continue,
        };

        let line = String::from_utf8_lossy(&message);
        println!("  [{}] {}", stream_type, line.trim());

        line_count += 1;
        if line_count >= 5 {
            break;
        }
    }

    // Test streaming logs
    println!("\nTesting streaming logs (3 second timeout):");
    test_streaming_logs(docker, &container_id).await?;

    Ok(())
}

async fn test_streaming_logs(docker: &Docker, container_id: &str) -> Result<()> {
    let options = Some(LogsOptions {
        stdout: true,
        stderr: true,
        follow: true,
        tail: "0".to_string(),
        timestamps: true,
        ..Default::default()
    });

    let mut stream = docker.logs(container_id, options);

    let result = tokio::time::timeout(tokio::time::Duration::from_secs(3), async {
        let mut count = 0;
        while let Some(log_result) = stream.next().await {
            let log_output = log_result?;

            let (stream_type, message) = match log_output {
                bollard::container::LogOutput::StdOut { message } => ("STDOUT", message),
                bollard::container::LogOutput::StdErr { message } => ("STDERR", message),
                _ => continue,
            };

            let line = String::from_utf8_lossy(&message);
            println!("  [STREAM-{}] {}", stream_type, line.trim());

            count += 1;
            if count >= 3 {
                break;
            }
        }
        Ok::<(), anyhow::Error>(())
    })
    .await;

    match result {
        Ok(_) => println!("  Streaming test completed"),
        Err(_) => println!("  No new logs in 3 seconds (normal)"),
    }

    Ok(())
}

async fn test_container_exec(docker: &Docker) -> Result<()> {
    let container_id = find_running_container(docker).await?;
    let container_id = match container_id {
        Some(id) => id,
        None => {
            println!("No running containers found");
            return Ok(());
        }
    };

    println!("Testing exec for container: {}", &container_id[..12]);

    // Test simple command
    let exec_options = CreateExecOptions {
        cmd: Some(vec!["echo", "Hello from bollard exec!"]),
        attach_stdout: Some(true),
        attach_stderr: Some(true),
        ..Default::default()
    };

    let exec = docker.create_exec(&container_id, exec_options).await?;

    match docker
        .start_exec(
            &exec.id,
            Some(StartExecOptions {
                detach: false,
                tty: false,
                output_capacity: None,
            }),
        )
        .await?
    {
        StartExecResults::Attached { mut output, .. } => {
            while let Some(result) = output.next().await {
                let log_output = result?;
                match log_output {
                    bollard::container::LogOutput::StdOut { message } => {
                        let line = String::from_utf8_lossy(&message);
                        println!("  [EXEC-OUT] {}", line.trim());
                    }
                    bollard::container::LogOutput::StdErr { message } => {
                        let line = String::from_utf8_lossy(&message);
                        println!("  [EXEC-ERR] {}", line.trim());
                    }
                    _ => {}
                }
            }
        }
        StartExecResults::Detached => {
            println!("  Command executed in detached mode");
        }
    }

    // Test interactive shell capability
    println!("\nTesting interactive shell setup:");
    let exec_options = CreateExecOptions {
        cmd: Some(vec!["/bin/sh"]),
        attach_stdin: Some(true),
        attach_stdout: Some(true),
        attach_stderr: Some(true),
        tty: Some(true),
        ..Default::default()
    };

    let exec = docker.create_exec(&container_id, exec_options).await?;
    println!("  ✓ Created interactive exec session: {}", &exec.id[..12]);
    println!("  ✓ TTY enabled, stdin/stdout/stderr attached");
    println!("  ✓ Ready for bidirectional communication");

    Ok(())
}

async fn find_running_container(docker: &Docker) -> Result<Option<String>> {
    let containers = docker
        .list_containers(Some(ListContainersOptions {
            all: false, // Only running
            ..Default::default()
        }))
        .await?;

    Ok(containers.first().and_then(|c| c.id.clone()))
}
