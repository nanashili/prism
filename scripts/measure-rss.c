/* Linux fresh-process RSS launcher. A small exec'd parent avoids inheriting the
 * Python fixture generator's resident/high-water memory in the measured child. */
#include <errno.h>
#include <stdio.h>
#include <stdlib.h>
#include <sys/resource.h>
#include <sys/types.h>
#include <sys/wait.h>
#include <unistd.h>

int main(int argc, char **argv) {
    if (argc < 3) {
        fprintf(stderr, "usage: measure-rss OUTPUT COMMAND [ARG...]\n");
        return 2;
    }
    pid_t child = fork();
    if (child == -1) {
        perror("fork");
        return 2;
    }
    if (child == 0) {
        execvp(argv[2], &argv[2]);
        perror("execvp");
        _exit(127);
    }
    int status;
    struct rusage usage;
    while (wait4(child, &status, 0, &usage) == -1) {
        if (errno != EINTR) {
            perror("wait4");
            return 2;
        }
    }
    FILE *output = fopen(argv[1], "w");
    if (!output) {
        perror("fopen");
        return 2;
    }
    fprintf(output, "{\"peak_rss_kib\":%ld}\n", usage.ru_maxrss);
    if (fclose(output)) {
        perror("fclose");
        return 2;
    }
    return WIFEXITED(status) ? WEXITSTATUS(status) : 128 + WTERMSIG(status);
}
