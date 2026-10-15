import SwiftUI

// sirno:witness:about-page:begin
struct AboutView: View {
    var body: some View {
        Group {
            switch AboutContent.bundled {
            case .success(let content):
                ScrollView {
                    VStack(alignment: .leading, spacing: 28) {
                        hero(content)
                        ForEach(content.sections) { section in
                            VStack(alignment: .leading, spacing: 14) {
                                Divider()
                                Text(section.title)
                                    .font(.headline)
                                    .accessibilityAddTraits(.isHeader)
                                ForEach(section.paragraphs, id: \.self) { paragraph in
                                    Text(paragraph)
                                        .font(.body)
                                        .foregroundStyle(.secondary)
                                        .fixedSize(horizontal: false, vertical: true)
                                        .textSelection(.enabled)
                                }
                                if let link = section.link {
                                    Link(destination: link.url) {
                                        HStack(spacing: 12) {
                                            Text(link.label).fontWeight(.semibold)
                                            Image(systemName: section.id == "support" ? "star" : "arrow.up.right")
                                                .accessibilityHidden(true)
                                        }
                                        .padding(.vertical, 12)
                                        .padding(.horizontal, section.id == "support" ? 16 : 0)
                                        .background(section.id == "support" ? Color.teal.opacity(0.12) : .clear,
                                            in: RoundedRectangle(cornerRadius: 12))
                                    }
                                    .tint(.teal)
                                }
                            }
                        }
                    }
                    .padding(24)
                    .frame(maxWidth: 640, alignment: .leading)
                    .frame(maxWidth: .infinity)
                }
            case .failure:
                ContentUnavailableView("About is unavailable", systemImage: "info.circle",
                    description: Text("The project information could not be loaded."))
            }
        }
        .navigationTitle(AboutContent.navigationTitle)
        .navigationBarTitleDisplayMode(.inline)
    }

    private func hero(_ content: AboutContent) -> some View {
        VStack(alignment: .leading, spacing: 20) {
            HStack(spacing: 14) {
                HStack(spacing: -12) {
                    ForEach(0..<3) { index in
                        Circle()
                            .fill(.teal.opacity(0.12))
                            .overlay(Circle().stroke(.teal, lineWidth: 2))
                            .frame(width: 28, height: 28)
                            .offset(y: index == 1 ? -6 : 0)
                    }
                }
                .accessibilityHidden(true)
                Text(content.name).font(.title2.weight(.semibold)).tracking(-1)
            }
            Text(content.tagline)
                .font(.largeTitle.weight(.semibold))
                .tracking(-1)
                .fixedSize(horizontal: false, vertical: true)
                .accessibilityAddTraits(.isHeader)
            Text(content.summary)
                .foregroundStyle(.secondary)
                .fixedSize(horizontal: false, vertical: true)
            ViewThatFits(in: .horizontal) {
                HStack(spacing: 8) { principles(content) }
                VStack(alignment: .leading, spacing: 8) { principles(content) }
            }
        }
        .padding(.vertical, 16)
    }

    @ViewBuilder
    private func principles(_ content: AboutContent) -> some View {
        ForEach(content.principles, id: \.self) { principle in
            Text(principle)
                .font(.caption)
                .foregroundStyle(.secondary)
                .padding(.horizontal, 10)
                .padding(.vertical, 6)
                .overlay(Capsule().stroke(.quaternary, lineWidth: 1))
        }
    }
}
// sirno:witness:about-page:end
