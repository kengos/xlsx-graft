<?php

declare(strict_types=1);

namespace XlsxGraft\Tests;

use PHPUnit\Framework\TestCase;
use XlsxGraft\Cli;
use XlsxGraft\CliNotFoundException;
use XlsxGraft\Diagnostic;
use XlsxGraft\Tests\Support\Engine;
use XlsxGraft\Verifier;

/**
 * The Verifier against the real CLI: what a workbook, the rules and the
 * policy make of a result (docs/sdk.md § Verifying in an SDK).
 */
final class VerifierTest extends TestCase
{
    protected function setUp(): void
    {
        Engine::cli();
    }

    public function testAMacroIsDeniedUnlessItsPartIsAllowed(): void
    {
        $verifier = new Verifier();
        $denied = $verifier->verify(Engine::fixture('macro.xlsm'));

        self::assertTrue($denied->failed());
        self::assertNull($denied->value);
        self::assertNull($denied->error);
        self::assertSame(['part_denied', 'part_denied'], array_map(static fn(Diagnostic $d): string => $d->code, $denied->diagnostics));
        self::assertNotNull($denied->diagnostics[0]->decidedBy);
        self::assertSame('template', $denied->diagnostics[0]->fix['in'] ?? null);

        $allowed = $verifier->verify(Engine::fixture('macro.xlsm'), allowParts: ['macro']);
        self::assertTrue($allowed->success);
        self::assertNull($allowed->value);
        self::assertNull($allowed->error);
    }

    public function testAFileThatIsNotThereIsIo(): void
    {
        $path = Engine::tempDir().'/missing.xlsx';
        $result = (new Verifier())->verify($path);

        self::assertTrue($result->failed());
        self::assertSame('io', $result->error?->code);
        self::assertStringContainsString($path, $result->error->message);
        rmdir(dirname($path));
    }

    public function testAFaultOfTheRulesIsADiagnosticAndNoError(): void
    {
        $verifier = new Verifier();
        $targets = $verifier->verify(Engine::fixture('table.xlsx'), targets: ['excel 1990']);
        $functions = $verifier->verify(Engine::fixture('table.xlsx'), allowFunctions: ['_xlfn.SEQUENCE']);
        $dash = $verifier->verify(Engine::fixture('table.xlsx'), targets: ['-x']);

        self::assertSame(['target_invalid', 'rules_invalid', 'target_invalid'], [
            $targets->diagnostics[0]->code,
            $functions->diagnostics[0]->code,
            $dash->diagnostics[0]->code,
        ]);
        self::assertSame([null, null, null], [$targets->error, $functions->error, $dash->error]);
        self::assertTrue($targets->failed());
    }

    public function testBestEffortIsAWarningAndFailOnWarnFailsOnIt(): void
    {
        $verifier = new Verifier();
        $warned = $verifier->verify(Engine::fixture('dynamic-array.xlsx'), bestEffort: ['excel 2016']);
        $failed = $verifier->verify(Engine::fixture('dynamic-array.xlsx'), bestEffort: ['excel 2016'], failOn: 'warn');

        self::assertTrue($warned->success);
        self::assertContains('warn', array_map(static fn(Diagnostic $d): string => $d->severity, $warned->diagnostics));
        self::assertTrue($failed->failed());
        self::assertNull($failed->error);
    }

    public function testAFailOnTheCliRefusesIsExitStatus(): void
    {
        foreach (['nope', ''] as $failOn) {
            $result = (new Verifier())->verify(Engine::fixture('table.xlsx'), failOn: $failOn);

            self::assertSame('exit_status', $result->error?->code, $failOn);
            self::assertStringStartsWith('the CLI exited with status 2: ', $result->error->message);
        }
    }

    public function testAValueThatIsNotUtf8IsExitStatus(): void
    {
        $result = (new Verifier())->verify(Engine::fixture('table.xlsx'), targets: ["\xff"]);

        self::assertTrue($result->failed());
        self::assertSame('exit_status', $result->error?->code);
        self::assertStringStartsWith('the CLI exited with status 2: ', $result->error->message);
    }

    public function testVerifiesBytes(): void
    {
        $verifier = new Verifier();
        $table = $verifier->verifyBytes((string) file_get_contents(Engine::fixture('table.xlsx')));
        $empty = $verifier->verifyBytes('');

        self::assertTrue($table->success);
        self::assertTrue($empty->failed());
        self::assertNull($empty->error);
        self::assertNotSame([], $empty->diagnostics);
    }

    public function testThePolicyCountsWithItsKeysInCamelCase(): void
    {
        $result = (new Verifier(policy: ['max_entries' => 1]))->verify(Engine::fixture('table.xlsx'));

        self::assertTrue($result->failed());
        self::assertSame('policy', $result->diagnostics[0]->decidedBy['source'] ?? null);
    }

    public function testAPolicyWithAnArrayInsideAndFailOnIsTakenByTheEngine(): void
    {
        $policy = ['forbid' => ['parts' => [['kind' => 'macro', 'reason' => 'no macros']]], 'fail_on' => 'warn'];
        $verifier = new Verifier(policy: $policy);

        self::assertTrue($verifier->verify(Engine::fixture('macro.xlsm'), allowParts: ['macro'])->failed());
        self::assertTrue($verifier->verify(Engine::fixture('dynamic-array.xlsx'), bestEffort: ['excel 2016'])->failed());
    }

    public function testAPolicyTheEngineDoesNotTakeIsFoundByTheFirstVerify(): void
    {
        $verifier = new Verifier(policy: ['max_entries' => 0, 'forbid' => ['parts' => [['kind' => 'nope', 'reason' => 'r']]]]);
        $result = $verifier->verify(Engine::fixture('table.xlsx'));

        self::assertTrue($result->failed());
        self::assertNull($result->error);
        self::assertSame(['policy_invalid', 'policy_invalid'], array_map(static fn(Diagnostic $d): string => $d->code, $result->diagnostics));
        self::assertSame(['maxEntries', 'forbid.parts[0].kind'], array_map(static fn(Diagnostic $d): mixed => $d->args['key'] ?? null, $result->diagnostics));
    }

    public function testNoCliIsFoundWhenTheVerifierIsMade(): void
    {
        $dir = Engine::tempDir();
        $cli = (string) getenv(Cli::ENV_VAR);
        $path = (string) getenv('PATH');
        putenv(Cli::ENV_VAR.'=');
        putenv("PATH={$dir}");
        try {
            new Verifier();
            self::fail('no CliNotFoundException');
        } catch (CliNotFoundException) {
            self::addToAssertionCount(1);
        } finally {
            putenv(Cli::ENV_VAR.'='.$cli);
            putenv("PATH={$path}");
            rmdir($dir);
        }
    }
}
