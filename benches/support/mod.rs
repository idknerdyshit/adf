pub(crate) struct Workload {
    pub(crate) name: &'static str,
    pub(crate) xml: String,
}

const TYPICAL_PROSPECT: &str = r#"<prospect status="new">
  <id sequence="1" source="benchmark">lead-0001</id>
  <requestdate>2026-07-15T12:00:00-04:00</requestdate>
  <vehicle interest="buy" status="used">
    <id source="vin">1BENCHMARK00000001</id>
    <year>2024</year>
    <make>Example</make>
    <model>Roadster</model>
    <vin>1BENCHMARK00000001</vin>
    <stock>STOCK-001</stock>
    <trim>Touring</trim>
    <odometer status="original" units="mi">12345</odometer>
    <colorcombination>
      <interiorcolor>Black</interiorcolor>
      <exteriorcolor>Blue</exteriorcolor>
      <preference>1</preference>
    </colorcombination>
    <price type="quote" currency="USD">24995</price>
    <option>
      <optionname>Example package</optionname>
      <manufacturercode>PKG1</manufacturercode>
      <price type="msrp" currency="USD">1200</price>
    </option>
    <comments>Fabricated vehicle data for parser benchmarks.</comments>
  </vehicle>
  <customer>
    <contact primarycontact="1">
      <name part="full" type="individual">Example Customer</name>
      <email preferredcontact="1">customer@example.test</email>
      <phone type="voice" time="day">555-0100</phone>
      <address type="home">
        <street>100 Benchmark Way</street>
        <city>Example City</city>
        <regioncode>MI</regioncode>
        <postalcode>48000</postalcode>
        <country>US</country>
      </address>
    </contact>
    <comments>Fabricated customer data for repeatable measurements.</comments>
  </customer>
  <vendor>
    <id source="benchmark">vendor-001</id>
    <vendorname>Example Motors</vendorname>
    <url>https://dealer.example.test</url>
    <contact>
      <name part="full">Example Sales</name>
      <email>sales@example.test</email>
    </contact>
  </vendor>
  <provider>
    <id source="benchmark">provider-001</id>
    <name part="full" type="business">Example Provider</name>
    <service>Benchmark Leads</service>
    <url>https://provider.example.test</url>
  </provider>
</prospect>"#;

const EXTENSION_MIXED: &str = r#"<!DOCTYPE adf [<!ENTITY partner "retained">]>
<adf xmlns:p="urn:example:partner" p:batch="benchmark">
  <!-- fabricated extension-heavy benchmark input -->
  <p:metadata p:source="synthetic">
    <p:payload><![CDATA[<offer><code>BENCH</code></offer>]]></p:payload>
    <p:label>Partner &amp; benchmark &partner;</p:label>
  </p:metadata>
  <prospect status="new" p:score="97">
    <id source="benchmark">extension-001</id>
    <requestdate>2026-07-15T12:00:00Z</requestdate>
    <vehicle interest="buy" p:inventory="external">
      <year>2025</year><make>Example</make><model>Extension</model>
      <p:pricing currency="USD"><p:amount>31000</p:amount></p:pricing>
      <comments><![CDATA[Text with <markup> kept as CDATA.]]></comments>
    </vehicle>
    <customer>
      <contact p:identity="fabricated">
        <name part="full">Extension Customer</name>
        <email>extension@example.test</email>
        <p:preference channel="email">daily</p:preference>
      </contact>
      <comments>Keep &partner; as an unresolved text entity.</comments>
    </customer>
    <vendor><vendorname>Extension Motors</vendorname></vendor>
  </prospect>
  <?partner benchmark?>
</adf>"#;

pub(crate) fn workloads() -> Vec<Workload> {
    vec![
        Workload {
            name: "minimal",
            xml: "<adf/>".to_owned(),
        },
        Workload {
            name: "typical",
            xml: build_batch(1),
        },
        Workload {
            name: "extension_mixed",
            xml: EXTENSION_MIXED.to_owned(),
        },
        Workload {
            name: "batch_100",
            xml: build_batch(100),
        },
        Workload {
            name: "batch_1000",
            xml: build_batch(1_000),
        },
    ]
}

fn build_batch(prospects: usize) -> String {
    let prospect = without_intertag_whitespace(TYPICAL_PROSPECT);
    let mut xml =
        String::with_capacity("<adf></adf>".len() + prospect.len().saturating_mul(prospects));
    xml.push_str("<adf>");
    for _ in 0..prospects {
        xml.push_str(&prospect);
    }
    xml.push_str("</adf>");
    xml
}

fn without_intertag_whitespace(input: &str) -> String {
    let mut compact = String::with_capacity(input.len());
    let mut remaining = input;

    while let Some(close) = remaining.find('>') {
        compact.push_str(&remaining[..=close]);
        remaining = &remaining[close + 1..];

        let Some(open) = remaining.find('<') else {
            compact.push_str(remaining);
            return compact;
        };
        let text = &remaining[..open];
        if !text.trim().is_empty() {
            compact.push_str(text);
        }
        remaining = &remaining[open..];
    }

    compact.push_str(remaining);
    compact
}
