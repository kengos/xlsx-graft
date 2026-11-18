<?php

declare(strict_types=1);

namespace XlsxGraft;

/**
 * One relationship of a package or of a part.
 */
final readonly class TreeRelationship
{
    /**
     * @param string $id the relationship's id (`rId1`)
     * @param string $type the relationship's type, a URI
     * @param TreeTarget $target the part or the external target the relationship points to
     */
    public function __construct(
        public string $id,
        public string $type,
        public TreeTarget $target,
    ) {}
}
