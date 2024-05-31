#include <stdio.h>
#include <pwd.h>
#include <errno.h>

// get-home <user>
// retrieve user's home directory full path

int main(int argc, char *argv[]) {
    errno = 0;
    if (argc == 1)
        return 1;

    struct passwd *pwd = getpwnam(argv[1]);

    if (pwd == NULL) {
        fprintf(stderr, "Error: can't retrieve user's home directory.\n");
        return 1;
    }

    printf("home: %s\n", pwd->pw_dir);

    if (errno != 0)
        printf("errno: %d\n", errno);
    return 0;
}
