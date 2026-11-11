<?php

declare(strict_types=1);

namespace XlsxGraft\Tests;

use PHPUnit\Framework\TestCase;
use XlsxGraft\Tests\Support\Engine;
use XlsxGraft\Verifier;
use XlsxGraft\VerifierPolicy;

/**
 * How the policy a Verifier is given is written as JSON (docs/sdk.md §
 * Verifying in an SDK).
 */
final class VerifierPolicyTest extends TestCase
{
    public function testStringKeysAreWrittenInCamelCaseAtEveryDepthAndValuesStay(): void
    {
        $policy = [
            'max_entries' => 1,
            'maxEntryBytes' => 2,
            'forbid' => ['functions' => [['name' => 'A_b', 'reason' => 'not_changed']], 'parts' => []],
            'deny_scheduled' => true,
            'ratio' => 1.0,
            'path' => 'a/b é',
            7 => 'x',
        ];

        self::assertSame(
            '{"maxEntries":1,"maxEntryBytes":2,"forbid":{"functions":[{"name":"A_b","reason":"not_changed"}],"parts":[]},"denyScheduled":true,"ratio":1.0,"path":"a/b é","7":"x"}',
            VerifierPolicy::json($policy),
        );
        self::assertSame('{"a1":[[1,2]],"a1B":0}', VerifierPolicy::json(['a_1' => [[1, 2]], 'a_1_b' => 0]));
    }

    public function testAKeyInAnAssociativeArrayInsideAListIsWrittenInCamelCase(): void
    {
        self::assertSame('{"a":{"bC":[{"dE":1}]}}', VerifierPolicy::json(['a' => ['b_c' => [['d_e' => 1]]]]));
    }

    public function testAnEmptyPolicyIsAnEmptyObjectAndAnEmptyArrayInsideItStaysAnArray(): void
    {
        self::assertSame('{}', VerifierPolicy::json([]));
        self::assertSame('{"forbid":[]}', VerifierPolicy::json(['forbid' => []]));
    }

    public function testAPolicyThatCannotBeWrittenAsJsonIsInvalidArgument(): void
    {
        $deep = [];
        for ($i = 0; $i < 600; $i++) {
            $deep = [$deep];
        }
        foreach ([['max_entries' => INF], ['max_entries' => NAN], ['name' => "\xff"], $deep, ['a' => $deep]] as $policy) {
            try {
                new Verifier($policy);
                self::fail('the policy was written');
            } catch (\InvalidArgumentException $e) {
                self::assertStringStartsWith('the policy cannot be written as JSON: ', $e->getMessage());
                self::assertInstanceOf(\JsonException::class, $e->getPrevious());
            }
        }
    }

    public function testChangingTheArrayAfterwardsChangesNothing(): void
    {
        Engine::cli();
        $policy = ['max_entries' => 1];
        $verifier = new Verifier($policy);
        $policy['max_entries'] = 1_000_000;

        self::assertSame('policy', $verifier->verify(Engine::fixture('table.xlsx'))->diagnostics[0]->decidedBy['source'] ?? null);
    }
}
