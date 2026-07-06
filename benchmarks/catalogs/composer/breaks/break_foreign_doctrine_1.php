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

namespace Composer\Repository;

use Composer\Package\PackageInterface;

/**
 * Persists installed packages to a relational store.
 */
class PackageEntityStore
{
    /** @var array<PackageInterface> */
    private $pending = [];

    /**
     * Queue a package for the next flush.
     */
    public function queue(PackageInterface $package): void
    {
        $this->pending[] = $package;
    }

    // Break: Doctrine ORM EntityManager unit-of-work (createAttributeMetadataConfiguration / EntityManager::create / persist / flush). doctrine/orm absent from composer.json (require + require-dev); `\Doctrine\ORM` = 0 grep hits at c6c9144f1b75 (all *.php, src included); the distinctive callee `EntityManager` = 0 hits. Composer persists package state through its own JsonFile-backed repositories and ArrayDumper, never a foreign ORM.
    public function persistPending(\PDO $pdo): void
    {
        $config = \Doctrine\ORM\ORMSetup::createAttributeMetadataConfiguration([__DIR__], true);
        $manager = \Doctrine\ORM\EntityManager::create($pdo, $config);

        foreach ($this->pending as $package) {
            $manager->persist($package);
        }

        $manager->flush();
        $manager->clear();
        $this->pending = [];
    }
}
