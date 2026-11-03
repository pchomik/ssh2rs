# Minimal local SSH/SFTP server for ssh2rs tests.
#
# Build: docker build -t ssh2rs-sshd .
# Run:   docker run --rm -p 2222:22 -e SSH_USER=test -e SSH_PASSWORD=secret ssh2rs-sshd
# Test:  SSH2RS_TEST_HOST=127.0.0.1 SSH2RS_TEST_PORT=2222 SSH2RS_TEST_USERNAME=test SSH2RS_TEST_PASSWORD=secret
#
# SSH_USER and SSH_PASSWORD default to "test" and "test".

FROM alpine:3.20

RUN apk add --no-cache openssh-server openssh-sftp-server \
 && ssh-keygen -A \
 && sed -i 's/^#*PasswordAuthentication.*/PasswordAuthentication yes/' /etc/ssh/sshd_config

EXPOSE 22

CMD ["sh", "-c", ": \"${SSH_USER:=test}\" \"${SSH_PASSWORD:=test}\" \
 && (id \"$SSH_USER\" >/dev/null 2>&1 || adduser -D \"$SSH_USER\") \
 && echo \"$SSH_USER:$SSH_PASSWORD\" | chpasswd \
 && exec /usr/sbin/sshd -D -e"]
