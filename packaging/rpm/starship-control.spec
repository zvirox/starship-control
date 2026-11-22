Name:           starship-control
Version:        0.1.0
Release:        1%{?dist}
Summary:        Native Linux AI Agent Orchestrator & Command Center
License:        Apache-2.0
URL:            https://github.com/zvirox/starship-control
Source0:        %{name}-%{version}.tar.gz

BuildRequires:  cargo
BuildRequires:  rust >= 1.80
BuildRequires:  nodejs
BuildRequires:  npm
BuildRequires:  webkit2gtk4.1-devel
BuildRequires:  gtk3-devel
BuildRequires:  libayatana-appindicator-devel
BuildRequires:  openssl-devel
BuildRequires:  sqlite-devel
BuildRequires:  pipewire-devel

Requires:       webkit2gtk4.1
Requires:       gtk3
Requires:       libayatana-appindicator
Requires:       openssl
Requires:       sqlite
Requires:       pipewire-libs

%description
Starship Control is a local-first, native Linux AI agent orchestrator and
unified command center for desktop applications (Easy Edits Pro, ScreenStudio Pro,
Audio Labs Pro, Net Watch Pro, and Agentic Tuneup Pro).

%prep
%autosetup -n %{name}-%{version}

%build
cd app
npm ci
npm run build
cd ..
cargo build --release --locked -p starship-control

%install
rm -rf %{buildroot}
install -Dm755 target/release/starship-control %{buildroot}%{_bindir}/starship-control
install -Dm644 packaging/starship-control.desktop %{buildroot}%{_datadir}/applications/starship-control.desktop

for size in 32x32 128x128 256x256 512x512; do
    install -Dm644 packaging/icons/hicolor/${size}/apps/starship-control.png \
        %{buildroot}%{_datadir}/icons/hicolor/${size}/apps/starship-control.png
done

%files
%license LICENSE
%doc README.md
%{_bindir}/starship-control
%{_datadir}/applications/starship-control.desktop
%{_datadir}/icons/hicolor/*/apps/starship-control.png

%post
/usr/bin/update-desktop-database &> /dev/null || :
/usr/bin/gtk-update-icon-cache %{_datadir}/icons/hicolor &>/dev/null || :

%postun
/usr/bin/update-desktop-database &> /dev/null || :
/usr/bin/gtk-update-icon-cache %{_datadir}/icons/hicolor &>/dev/null || :

%changelog
* Fri Nov 21 2026 Starship Control Team <team@starshipcontrol.dev> - 0.1.0-1
- Initial release of Starship Control native Linux agent orchestrator
