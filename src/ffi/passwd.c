#include <unistd.h>
#include <pwd.h>

char *get_home_dir(char *username) {
    struct passwd *pwd = getpwnam(username);

    if (pwd == NULL)
        return NULL;

    return pwd->pw_dir;
}
