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

namespace Composer\Command;

use League\Csv\Writer;

/**
 * Exports the resolved license table as CSV for `composer licenses`.
 */
class LicenseCsvExporter
{
    /** @var list<array{name: string, license: string}> */
    private $rows = [];

    public function add(string $name, string $license): void
    {
        $this->rows[] = ['name' => $name, 'license' => $license];
    }

    // Break: league/csv document building reached through a receiver ($writer = Writer::createFromString(''); $writer->insertOne(...)); the foreign dependency is a bare `use League\Csv\Writer` submodule import, not an FQN in call position. league/csv absent from composer.json (require + require-dev); `League\Csv` = 0 grep hits at c6c9144f1b75 (all *.php, src included); the distinctive callee `insertOne` = 0 hits. Composer renders tabular command output through Symfony Console's Table helper, never a foreign CSV library.
    public function export(): string
    {
        $writer = Writer::createFromString('');
        $writer->insertOne(['name', 'license']);
        foreach ($this->rows as $row) {
            $writer->insertOne([$row['name'], $row['license']]);
        }

        return $writer->toString();
    }
}
