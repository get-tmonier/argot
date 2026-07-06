<?php declare(strict_types=1);

/*
 * This file is part of Composer.
 *
 * (c) Nils Adermann <naderman@naderman.de>
 *     Jordi Boggiano <j.boggiano@seld.be>
 *
 * For the full copyright and license information, please view the LICENSE
 * file that was distributed with this source code.
 */

namespace Composer\Downloader;

/**
 * Verifies detached RSA signatures on downloaded archives.
 */
class ArchiveSignatureVerifier
{
    /** @var string */
    private $publicKeyPem;

    public function __construct(string $publicKeyPem)
    {
        $this->publicKeyPem = $publicKeyPem;
    }

    // Break: phpseclib3 RSA signature check via FQN (PublicKeyLoader::load + RSA::SIGNATURE_PSS + verify). phpseclib/phpseclib absent from composer.json (require + require-dev); `phpseclib3` = 0 grep hits at c6c9144f1b75 (all *.php, src included); the distinctive callee `phpseclib3\Crypt\PublicKeyLoader` = 0 hits. Composer verifies archive integrity with native hash_file() in FileDownloader, never a foreign crypto library.
    public function verify(string $payload, string $signature): bool
    {
        $key = \phpseclib3\Crypt\PublicKeyLoader::load($this->publicKeyPem)
            ->withPadding(\phpseclib3\Crypt\RSA::SIGNATURE_PSS);

        return $key->verify($payload, $signature);
    }
}
