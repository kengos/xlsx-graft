<?php

declare(strict_types=1);

namespace XlsxGraft;

/**
 * Runs the engine's CLI once (docs/sdk.md § Running the CLI).
 *
 * The command is an array, so `proc_open` starts the CLI itself and no shell
 * reads the arguments. stdin is closed at once, so a CLI that ever asked for
 * input reads its end rather than waiting. stderr goes to a temporary file
 * rather than a second pipe: reading two pipes one after the other stops the
 * moment the unread one fills, and polling both is what `stream_select`
 * cannot do on Windows. The run is waited for with `proc_get_status`, the one
 * place that tells a CLI killed by a signal from one that exited:
 * `proc_close` returns the signal's number as if it were a status.
 *
 * @internal
 */
final class CliProcess
{
    /**
     * @param list<string> $args
     *
     * @return array{int, int|null, string, string} the exit status (-1 when killed by a signal), the
     *                                               signal that killed it, stdout and stderr
     *
     * @throws CliNotFoundException when the CLI cannot be run, or no
     *                              temporary file can take its stderr
     */
    public static function run(string $cli, array $args): array
    {
        // A disabled function does not exist in PHP 8, so calling it would
        // throw an Error. Neither throw can happen in the suite's own PHP, so
        // each sits on its condition's line and a test runs it in a child PHP
        // set up to fail it.
        function_exists('proc_open') || throw new CliNotFoundException('this PHP cannot run the CLI: proc_open() is disabled (disable_functions)');
        $stderr = @tmpfile() ?: throw new CliNotFoundException('this PHP cannot run the CLI: no temporary file can take its stderr (sys_temp_dir: '.sys_get_temp_dir().')');
        $pipes = [];
        error_clear_last();
        $process = @proc_open([$cli, ...$args], [['pipe', 'r'], ['pipe', 'w'], $stderr], $pipes);
        if ($process === false) {
            $why = error_get_last()['message'] ?? 'proc_open() failed';
            fclose($stderr);

            throw new CliNotFoundException(Cli::installHint("{$cli} could not be run ({$why})"));
        }

        fclose($pipes[0]);
        $stdout = (string) stream_get_contents($pipes[1]);
        fclose($pipes[1]);
        // stdout closes before the CLI exits, so its status may not be in yet.
        while (($state = proc_get_status($process))['running']) {
            usleep(1000);
        }
        proc_close($process);

        rewind($stderr);
        $errors = (string) stream_get_contents($stderr);
        fclose($stderr);

        return [$state['exitcode'], $state['signaled'] ? $state['termsig'] : null, $stdout, $errors];
    }
}
