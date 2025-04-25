#include <stdio.h>
#include <stdlib.h>

int foo(x, y) {
    return x + y;
}

char *concat(char *str_1, char *str_2) {
    int len1, len2 = 0;
    while (str_1[len1] != '\0') len1++;
    while (str_2[len2] != '\0') len2++;

    char *buffer = malloc(len1 + len2 * sizeof(char) + 1);
    
    for (int i = 0; i < len1; i++) {
        buffer[i] = str_1[i];
    }

    for (int j = len1; j < len2; j++) {
        buffer[j] = str_2[j - len1];
    }

    buffer[len1 + len2] = '\0';
    return buffer;
}

int main(void) {
    char *str1 = "foo";
    char *str2 = "bar";

    char *join = concat(str1, str2);
    printf("%s", join);

    return 0;
}
