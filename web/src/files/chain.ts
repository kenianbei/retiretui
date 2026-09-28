/** Where a scenario's `base` puts its base: beside it, in the flat workspace. */
export function basePath(base: string): string {
  return base.startsWith("/") ? base : `/${base.replace(/^\.\//, "")}`;
}

/** What each file's `base` names, where it names one. */
export type BaseAt = (path: string) => string | undefined;

/** The files of `paths` whose `base` is `path`. */
export function scenariosOver(
  path: string,
  paths: readonly string[],
  baseAt: BaseAt,
): string[] {
  return paths.filter((each) => {
    const base = baseAt(each);
    return base !== undefined && basePath(base) === path;
  });
}

/** The files of `paths` built on `path`, directly or through one another. */
export function builtOn(
  path: string,
  paths: readonly string[],
  baseAt: BaseAt,
): string[] {
  const found: string[] = [];
  let over = [path];
  while (over.length > 0) {
    over = over
      .flatMap((base) => scenariosOver(base, paths, baseAt))
      .filter((each) => each !== path && !found.includes(each));
    found.push(...over);
  }
  return [...new Set(found)];
}
