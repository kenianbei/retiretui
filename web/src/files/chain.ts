/** The file each file is a scenario over, where it is one. */
export type BaseAt = (path: string) => string | undefined;

/** The files of `paths` that are scenarios over `path`. */
export function scenariosOver(
  path: string,
  paths: readonly string[],
  baseAt: BaseAt,
): string[] {
  return paths.filter((each) => baseAt(each) === path);
}

/** The files of `paths` built on `path`, directly or through one another. */
export function builtOn(
  path: string,
  paths: readonly string[],
  baseAt: BaseAt,
): string[] {
  const found = new Set<string>();
  let over = [path];
  while (over.length > 0) {
    over = over
      .flatMap((base) => scenariosOver(base, paths, baseAt))
      .filter((each) => each !== path && !found.has(each));
    over.forEach((each) => found.add(each));
  }
  return [...found];
}
