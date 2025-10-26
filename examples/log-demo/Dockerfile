# Simple Dockerfile for log generator demo
FROM alpine:3.18

# Install bash (Alpine uses ash by default)
RUN apk add --no-cache bash

# Create a directory for our app
WORKDIR /app

# Copy the log generator script
COPY log-generator.sh /app/log-generator.sh

# Make sure the script is executable
RUN chmod +x /app/log-generator.sh

# Add a non-root user for security
RUN addgroup -g 1001 -S appgroup && \
    adduser -u 1001 -S appuser -G appgroup

# Change ownership of the app directory
RUN chown -R appuser:appgroup /app

# Switch to non-root user
USER appuser

# Set the default command
CMD ["/app/log-generator.sh"]

# Add labels for documentation
LABEL maintainer="Scotty Demo"
LABEL description="Log generator for testing scotty logs functionality"
LABEL version="1.0"