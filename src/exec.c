#include "../gekkota.h"

/* TODO: add stdin support, not sure how to implement not blocking */

void handle_out_line(char *line){
    printf("\x1b[0m>'\x1b[36m%s\x1b[0m'\n", line);
}

void handle_exit(int exit_code){
    printf("\x1b[33mwe got an exit code '%d'\n\x1b[0m", exit_code);
}

/* loop through the bytes of the file until a new lihne is found, or it
 * non-blockingly returns nothing 
 *
 * return is if the line is complete or not. in the case of a complete line, the
 * index is set back to 0 before this function returns
 */
int read_chunk(FILE *stream, char buff[], int *idxp){
    char c;
    int r, found = 0;
    while(1){
        r = fread(&c, 1, 1, stream);
        if(r > 0){
            found = 1;
            buff[(*idxp)++] = c;
        }else if(r == -1){
            return r;
        }else{
            return found;
        }
    }
}

/* 
 * fork and exec and redirect the pipes so that stdout/stderr of the child are
 * managable from the parent
 *
 * loop through the status of hte child and if any content is available from
 * the output streams
 */
int gka_exec(char *cmd_path, char *args[]){
    char out_buff[4096];
    int r;

    int out[2];
    pipe(out);

    int pid = fork();
    int status;
    int w;
    
    struct timespec interval;
    interval.tv_sec = 0;
    interval.tv_nsec = INTERVAL;

    int return_code;

    struct timespec time_remaining;

    if(pid == -1){
        fprintf(stderr, "Error forking");
        exit(1);
    }else if(pid == 0){
        dup2(out[1], 1);

        execvp(cmd_path, args);
    }else{
        fcntl(out[0], F_SETFL, O_NONBLOCK);

        FILE *fout= fdopen(out[0], "r");

        /* indexes used to keep track of where in the buffers the different
         * streams are 
         */
        int out_idx = 0;

        while(1){
            w = waitpid(pid, &status, WNOHANG);
            if (w != -1) {
                r = read_chunk(fout, out_buff, &out_idx);
                if(r > 0){
                    out_buff[out_idx] = '\0';
                    out_idx = 0;
                    handle_out_line(out_buff);
                }
                if(r != -1){
                    nanosleep(&interval, &time_remaining);
                    continue;
                }
            }
            if(WIFEXITED(status)){
               handle_exit(WEXITSTATUS(status));
               break; 
            }else if(WIFSIGNALED(status)){
               handle_exit(WTERMSIG(status));
               break;
            }else if(WIFSTOPPED(status)){
               handle_exit(WSTOPSIG(status));
               break;
            }
            nanosleep(&interval, &time_remaining);
        }
    }
}
