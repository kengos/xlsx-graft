<?php

declare(strict_types=1);

namespace XlsxGraft\Tests;

use PHPUnit\Framework\TestCase;
use XlsxGraft\Tests\Support\Engine;

/**
 * The package downloads nothing and depends on nothing (docs/sdk.md § Finding
 * the engine library).
 */
final class PackageTest extends TestCase
{
    // A call of a networking function, or a stream opened on a URL.
    private const NETWORKING = '/\b(?:curl_\w+|fsockopen|pfsockopen|stream_socket_client|socket_create|get_headers|ftp_connect)\s*\(|[\'"](?:https?|ftps?|ssl|tls|tcp|udp):\/\//';

    public function testUsesNoNetworking(): void
    {
        $sources = glob(Engine::PACKAGE_ROOT.'/src/*.php') ?: [];
        // A sweep over no files reports no offenders and proves nothing.
        self::assertContains(Engine::PACKAGE_ROOT.'/src/CliProcess.php', $sources);

        $offenders = array_filter($sources, static fn(string $file): bool => preg_match(self::NETWORKING, (string) file_get_contents($file)) === 1);
        self::assertSame([], array_values($offenders));
    }

    public function testTheSweepMatchesNetworking(): void
    {
        self::assertMatchesRegularExpression(self::NETWORKING, 'curl_init($url);');
        self::assertMatchesRegularExpression(self::NETWORKING, '$s = fsockopen("example.com", 80);');
        self::assertMatchesRegularExpression(self::NETWORKING, "file_get_contents('https://example.com/engine');");
        self::assertMatchesRegularExpression(self::NETWORKING, 'fopen("tcp://example.com:80", "r");');
        self::assertDoesNotMatchRegularExpression(self::NETWORKING, '$pipes = []; proc_open([$cli], $descriptors, $pipes);');
    }

    public function testRequiresPhpAloneAndRunsNoScripts(): void
    {
        $manifest = json_decode((string) file_get_contents(Engine::PACKAGE_ROOT.'/composer.json'), true);
        self::assertIsArray($manifest);

        self::assertSame(['php' => '>=8.3'], $manifest['require']);
        self::assertArrayNotHasKey('scripts', $manifest);
        // Packagist takes the version from the tag; the gate writes the
        // engine's into the archived copy (mk/sdk/php.mk).
        self::assertArrayNotHasKey('version', $manifest);
    }
}
