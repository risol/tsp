interface HeaderProps {
  title: string;
  subtitle?: string;
}

export function Header({ title, subtitle }: HeaderProps) {
  return (
    <>
      <h1>{title}</h1>
      {subtitle && <p class="subtitle">{subtitle}</p>}
    </>
  );
}

export default Header;
