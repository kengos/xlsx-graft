<?php

declare(strict_types=1);

namespace XlsxGraft;

/**
 * The parts of a workbook and the relationships between them, the value of
 * {@see Viewer::tree()}. Names and text are made for display (control
 * characters removed, long text cut), so they are not exact.
 */
final readonly class Tree
{
    /**
     * @param bool $complete whether every part is listed
     * @param bool $relationshipsKnown whether the package's relationships could be read
     * @param list<TreeRelationship> $relationships the package's own relationships
     * @param list<TreePart> $parts the parts
     */
    public function __construct(
        public bool $complete,
        public bool $relationshipsKnown,
        public array $relationships,
        public array $parts,
    ) {}
}
