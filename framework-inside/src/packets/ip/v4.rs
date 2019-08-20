use common::Result;
use native::mbuf::MBuf;
use packets::checksum::PseudoHeader;
use packets::ip::{IpAddrMismatchError, IpPacket, ProtocolNumber};
use packets::{buffer, Ethernet, Fixed, Header, Packet};
use std::fmt;
use std::net::{IpAddr, Ipv4Addr};
// use std::io::stdout;
// use std::io::Write;
use std::slice;

/*  From https://tools.ietf.org/html/rfc791#section-3.1
    Internet Datagram Header

     0                   1                   2                   3
     0 1 2 3 4 5 6 7 8 9 0 1 2 3 4 5 6 7 8 9 0 1 2 3 4 5 6 7 8 9 0 1
    +-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+
    |Version|  IHL  |    DSCP_ECN   |          Total Length         |
    +-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+
    |         Identification        |Flags|      Fragment Offset    |
    +-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+
    |  Time to Live |    Protocol   |         Header Checksum       |
    +-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+
    |                       Source Address                          |
    +-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+
    |                    Destination Address                        |
    +-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+
    |                    Options                    |    Padding    |
    +-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+

    Version:  4 bits
        The Version field indicates the format of the internet header.  This
        document describes version 4.

    IHL:  4 bits
        Internet Header Length is the length of the internet header in 32
        bit words, and thus points to the beginning of the data.  Note that
        the minimum value for a correct header is 5.

    DSCP_ECN:  8 bits
        Differentiated services (via RFC 2474 ~ https://tools.ietf.org/html/rfc2474)
        enhancements to the Internet protocol are intended to enable scalable
        service discrimination in the Internet without the need for per-flow
        state and signaling at every hop.  A variety of services may be built
        from a small, well-defined set of building blocks which are deployed in
        network nodes. The services may be either end-to-end or intra-domain;
        they include both those that can satisfy quantitative performance
        requirements (e.g., peak bandwidth) and those based on relative
        performance (e.g., "class" differentiation).

        Taking the last two bits, is ECN, the addition of Explicit
        Congestion Notification to IP; RFC-3168
        (https://tools.ietf.org/html/rfc3168) covers this in detail.
        This uses an ECN field in the IP header with two bits, making four ECN
        codepoints, '00' to '11'.  The ECN-Capable Transport (ECT) codepoints
        '10' and '01' are set by the data sender to indicate that the end-points
        of the transport protocol are ECN-capable; we call them ECT(0) and
        ECT(1) respectively.  The phrase "the ECT codepoint" in this documents
        refers to either of the two ECT codepoints.  Routers treat the ECT(0)
        and ECT(1) codepoints as equivalent.  Senders are free to use either the
        ECT(0) or the ECT(1) codepoint to indicate ECT, on a packet-by-packet
        basis.

    Total Length:  16 bits
        Total Length is the length of the datagram, measured in octets,
        including internet header and data.

    Identification:  16 bits
        An identifying value assigned by the sender to aid in assembling the
        fragments of a datagram.

    Flags:  3 bits
        Various Control Flags.

        Bit 0: reserved, must be zero
        Bit 1: (DF) 0 = May Fragment,  1 = Don't Fragment.
        Bit 2: (MF) 0 = Last Fragment, 1 = More Fragments.

          0   1   2
        +---+---+---+
        |   | D | M |
        | 0 | F | F |
        +---+---+---+

    Fragment Offset:  13 bits
        This field indicates where in the datagram this fragment belongs.
        The fragment offset is measured in units of 8 octets (64 bits).  The
        first fragment has offset zero.

    Time to Live:  8 bits
        This field indicates the maximum time the datagram is allowed to
        remain in the internet system.  If this field contains the value
        zero, then the datagram must be destroyed.  This field is modified
        in internet header processing.  The time is measured in units of
        seconds, but since every module that processes a datagram must
        decrease the TTL by at least one even if it process the datagram in
        less than a second, the TTL must be thought of only as an upper
        bound on the time a datagram may exist.  The intention is to cause
        undeliverable datagrams to be discarded, and to bound the maximum
        datagram lifetime.

    Protocol:  8 bits
        This field indicates the next level protocol used in the data
        portion of the internet datagram.  The values for various protocols
        are specified in "Assigned Numbers".

    Header Checksum:  16 bits
        A checksum on the header only.  Since some header fields change
        (e.g., time to live), this is recomputed and verified at each point
        that the internet header is processed.

    Source Address:  32 bits
        The source address.

    Destination Address:  32 bits
        The destination address.

    Options:  variable
        The options may appear or not in datagrams.  They must be
        implemented by all IP modules (host and gateways).  What is optional
        is their transmission in any particular datagram, not their
        implementation.
*/

// Masks
const DSCP: u8 = 0b1111_1100;
const ECN: u8 = !DSCP;

// Flags
const FLAGS_DF: u16 = 0b0100_0000_0000_0000;
const FLAGS_MF: u16 = 0b0010_0000_0000_0000;

/// IPv4 header
///
/// The header only include the fixed portion of the IPv4 header.
/// Options are parsed separately.
#[derive(Debug, Copy, Clone)]
#[repr(C)]
pub struct Ipv4Header {
    version_ihl: u8,
    dscp_ecn: u8,
    total_length: u16,
    identification: u16,
    flags_to_frag_offset: u16,
    ttl: u8,
    protocol: u8,
    checksum: u16,
    src: Ipv4Addr,
    dst: Ipv4Addr,
}

impl Ipv4Header {
    // the length of the ipv4 header;
    #[inline]
    fn ipv4_header_len(&self) -> usize {
        (self.ihl() << 2) as usize
    }

    #[inline]
    pub fn version(&self) -> u8 {
        // Protocol Version, should always be `4`
        (self.version_ihl & 0xf0) >> 4
    }

    #[inline]
    pub fn ihl(&self) -> u8 {
        self.version_ihl & 0x0f
    }

    #[inline]
    pub fn set_version_ihl(&mut self, version_ihl: u8) {
        self.version_ihl = version_ihl;
    }

    #[inline]
    pub fn total_length(&self) -> u16 {
        u16::from_be(self.total_length)
    }

    #[inline]
    fn set_total_length(&mut self, total_length: u16) {
        self.total_length = u16::to_be(total_length);
    }

    #[inline]
    pub fn identification(&self) -> u16 {
        u16::from_be(self.identification)
    }

    #[inline]
    pub fn set_identification(&mut self, identification: u16) {
        self.identification = u16::to_be(identification);
    }

    #[inline]
    pub fn ttl(&self) -> u8 {
        self.ttl
    }

    #[inline]
    pub fn set_ttl(&mut self, ttl: u8) {
        self.ttl = ttl;
    }

    #[inline]
    pub fn checksum(&self) -> u16 {
        u16::from_be(self.checksum)
    }

    #[allow(dead_code)]
    #[inline]
    fn set_checksum(&mut self, checksum: u16) {
        self.checksum = u16::to_be(checksum);
    }

    #[inline]
    pub fn protocol(&self) -> ProtocolNumber {
        ProtocolNumber::new(self.protocol)
    }

    #[inline]
    pub fn set_protocol(&mut self, protocol: ProtocolNumber) {
        self.protocol = protocol.0;
    }

    #[inline]
    pub fn src(&self) -> Ipv4Addr {
        self.src
    }

    #[inline]
    pub fn set_src(&mut self, src: Ipv4Addr) {
        self.src = src;
    }

    #[inline]
    pub fn dst(&self) -> Ipv4Addr {
        self.dst
    }

    #[inline]
    pub fn set_dst(&mut self, dst: Ipv4Addr) {
        self.dst = dst;
    }

    #[inline]
    pub fn init(&mut self, src: Ipv4Addr, dst: Ipv4Addr, proto: ProtocolNumber, total_length: u16){
        self.set_version_ihl(0x45);
        self.set_total_length(total_length);
        self.set_protocol(proto);
        self.set_src(src);
        self.set_dst(dst);
    }
}

impl Default for Ipv4Header {
    fn default() -> Ipv4Header {
        Ipv4Header {
            version_ihl: 4 << 4,
            dscp_ecn: 0,
            total_length: 0,
            identification: 0,
            flags_to_frag_offset: 0,
            ttl: 0,
            protocol: 0,
            checksum: 0,
            src: Ipv4Addr::UNSPECIFIED,
            dst: Ipv4Addr::UNSPECIFIED,
        }
    }
}

impl Header for Ipv4Header {}

/// IPv4 packet
#[derive(Debug)]
pub struct Ipv4 {
    envelope: Ethernet,
    mbuf: *mut MBuf,
    offset: usize,// this offset is the ipv4 header offset relative to the ethernet header
    header: *mut Ipv4Header,
}

impl Ipv4 {
    // the length of the whole ethernet packet;
    // does not include the 4-byte FCS part. 
    #[inline]
    fn data_len(&self) -> usize {
        unsafe { (*self.mbuf).pkt_len() }
    }

    // the length of the ipv4 header;
    #[inline]
    fn ipv4_header_len(&self) -> usize {
        (self.ihl() << 2) as usize
    }

    // the length of the ipv4 payload
    #[inline]
    fn payload_len(&self) -> usize {
        self.data_len() - self.offset - self.ipv4_header_len()
    }

    #[inline]
    fn payload(&self) -> *mut u8 {
        unsafe { (*self.mbuf).data_address(self.offset + self.ipv4_header_len()) }
    }

    #[inline]
    pub fn get_payload(&self) -> &[u8] {
        unsafe {
            // println!("{} {} {} {}", self.data_len(), self.offset, self.ipv4_header_len(), self.payload_len());
            // stdout().flush();
            let len = self.payload_len();
            slice::from_raw_parts(self.payload(), len)
        }
    }

    #[inline]
    pub fn get_payload_mut(&self) -> &mut [u8] {
        unsafe {
            // println!("{} {} {} {}", self.data_len(), self.offset, self.ipv4_header_len(), self.payload_len());
            // stdout().flush();
            let len = self.payload_len();
            slice::from_raw_parts_mut(self.payload(), len)
        }
    }

    #[inline]
    pub fn version(&self) -> u8 {
        // Protocol Version, should always be `4`
        (self.header().version_ihl & 0xf0) >> 4
    }

    #[inline]
    pub fn ihl(&self) -> u8 {
        self.header().version_ihl & 0x0f
    }

    #[inline]
    pub fn set_ihl(&mut self, ihl: u8) {
        self.header_mut().version_ihl = (self.header().version_ihl & 0x0f) | (ihl & 0x0f);
    }

    #[inline]
    pub fn dscp(&self) -> u8 {
        self.header().dscp_ecn >> 2
    }

    #[inline]
    pub fn set_dscp(&mut self, dscp: u8) {
        self.header_mut().dscp_ecn = (self.header().dscp_ecn & ECN) | (dscp << 2);
    }

    #[inline]
    pub fn ecn(&self) -> u8 {
        self.header().dscp_ecn & ECN
    }

    #[inline]
    pub fn set_ecn(&mut self, ecn: u8) {
        self.header_mut().dscp_ecn = (self.header().dscp_ecn & DSCP) | (ecn & ECN);
    }

    #[inline]
    pub fn total_length(&self) -> u16 {
        u16::from_be(self.header().total_length)
    }

    #[inline]
    fn set_total_length(&mut self, total_length: u16) {
        self.header_mut().total_length = u16::to_be(total_length);
    }

    #[inline]
    pub fn identification(&self) -> u16 {
        u16::from_be(self.header().identification)
    }

    #[inline]
    pub fn set_identification(&mut self, identification: u16) {
        self.header_mut().identification = u16::to_be(identification);
    }

    #[inline]
    pub fn dont_fragment(&self) -> bool {
        u16::from_be(self.header().flags_to_frag_offset) & FLAGS_DF != 0
    }

    #[inline]
    pub fn set_dont_fragment(&mut self) {
        self.header_mut().flags_to_frag_offset =
            u16::to_be(u16::from_be(self.header().flags_to_frag_offset) | FLAGS_DF);
    }

    #[inline]
    pub fn unset_dont_fragment(&mut self) {
        self.header_mut().flags_to_frag_offset =
            u16::to_be(u16::from_be(self.header().flags_to_frag_offset) & !FLAGS_DF);
    }

    #[inline]
    pub fn more_fragments(&self) -> bool {
        u16::from_be(self.header().flags_to_frag_offset) & FLAGS_MF != 0
    }

    #[inline]
    pub fn set_more_fragments(&mut self) {
        self.header_mut().flags_to_frag_offset =
            u16::to_be(u16::from_be(self.header().flags_to_frag_offset) | FLAGS_MF);
    }

    #[inline]
    pub fn unset_more_fragments(&mut self) {
        self.header_mut().flags_to_frag_offset =
            u16::to_be(u16::from_be(self.header().flags_to_frag_offset) & !FLAGS_MF);
    }

    #[inline]
    pub fn clear_flags(&mut self) {
        self.header_mut().flags_to_frag_offset =
            u16::to_be(u16::from_be(self.header().flags_to_frag_offset) & !0xe000);
    }

    #[inline]
    pub fn fragment_offset(&self) -> u16 {
        u16::from_be(self.header().flags_to_frag_offset) & 0x1fff
    }

    #[inline]
    pub fn set_fragment_offset(&mut self, offset: u16) {
        self.header_mut().flags_to_frag_offset = u16::to_be(
            (u16::from_be(self.header().flags_to_frag_offset) & 0xe000) | (offset & 0x1fff),
        );
    }

    #[inline]
    pub fn ttl(&self) -> u8 {
        self.header().ttl
    }

    #[inline]
    pub fn set_ttl(&mut self, ttl: u8) {
        self.header_mut().ttl = ttl;
    }

    #[inline]
    pub fn protocol(&self) -> ProtocolNumber {
        ProtocolNumber::new(self.header().protocol)
    }

    #[inline]
    pub fn set_protocol(&mut self, protocol: ProtocolNumber) {
        self.header_mut().protocol = protocol.0;
    }

    #[inline]
    pub fn checksum(&self) -> u16 {
        u16::from_be(self.header().checksum)
    }

    #[allow(dead_code)]
    #[inline]
    fn set_checksum(&mut self, checksum: u16) {
        self.header_mut().checksum = u16::to_be(checksum);
    }

    #[inline]
    pub fn src(&self) -> Ipv4Addr {
        self.header().src
    }

    #[inline]
    pub fn set_src(&mut self, src: Ipv4Addr) {
        self.header_mut().src = src;
    }

    #[inline]
    pub fn dst(&self) -> Ipv4Addr {
        self.header().dst
    }

    #[inline]
    pub fn set_dst(&mut self, dst: Ipv4Addr) {
        self.header_mut().dst = dst;
    }
}

impl fmt::Display for Ipv4 {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        write!(
            f,
            "{} > {} version: {}, ihl: {}, dscp: {}, ecn: {}, len: {}, dont_fragment: {}, more_fragments: {}, fragment_offset: {}, ttl: {}, protocol: {}, checksum: {}",
            self.src(),
            self.dst(),
            self.version(),
            self.ihl(),
            self.dscp(),
            self.ecn(),
            self.total_length(),
            self.dont_fragment(),
            self.more_fragments(),
            self.fragment_offset(),
            self.ttl(),
            self.protocol(),
            self.checksum()
        )
    }
}

impl Packet for Ipv4 {
    type Header = Ipv4Header;
    type Envelope = Ethernet;

    #[inline]
    fn envelope(&self) -> &Self::Envelope {
        &self.envelope
    }

    #[inline]
    fn envelope_mut(&mut self) -> &mut Self::Envelope {
        &mut self.envelope
    }

    #[doc(hidden)]
    #[inline]
    fn mbuf(&self) -> *mut MBuf {
        self.mbuf
    }

    #[inline]
    fn offset(&self) -> usize {
        self.offset
    }

    #[doc(hidden)]
    #[inline]
    fn header(&self) -> &Self::Header {
        unsafe { &(*self.header) }
    }

    #[doc(hidden)]
    #[inline]
    fn header_mut(&mut self) -> &mut Self::Header {
        unsafe { &mut (*self.header) }
    }

    #[inline]
    fn header_len(&self) -> usize {
        Self::Header::size()
        // (self.ihl() << 2) as usize
    }

    #[doc(hidden)]
    #[inline]
    fn do_parse(envelope: Self::Envelope) -> Result<Self> {
        let mbuf = envelope.mbuf();
        let offset = envelope.payload_offset();
        let header = buffer::read_item::<Self::Header>(mbuf, offset)?;

        Ok(Ipv4 {
            envelope,
            mbuf,
            offset,
            header,
        })
    }

    #[doc(hidden)]
    #[inline]
    fn do_push(envelope: Self::Envelope) -> Result<Self> {
        let mbuf = envelope.mbuf();
        let offset = envelope.payload_offset();

        buffer::alloc(mbuf, offset, Self::Header::size())?;
        let header = buffer::write_item::<Self::Header>(mbuf, offset, &Default::default())?;

        Ok(Ipv4 {
            envelope,
            mbuf,
            offset,
            header,
        })
    }

    #[inline]
    fn remove(self) -> Result<Self::Envelope> {
        buffer::dealloc(self.mbuf, self.offset, self.header_len())?;
        Ok(self.envelope)
    }

    #[inline]
    fn cascade(&mut self) {
        // TODO: fix header checksum
        let len = self.len() as u16;
        self.set_total_length(len);
        self.envelope_mut().cascade();
    }

    #[inline]
    fn deparse(self) -> Self::Envelope {
        self.envelope
    }
}

impl IpPacket for Ipv4 {
    #[inline]
    fn next_proto(&self) -> ProtocolNumber {
        self.protocol()
    }

    #[inline]
    fn src(&self) -> IpAddr {
        IpAddr::V4(self.src())
    }

    #[inline]
    fn set_src(&mut self, src: IpAddr) -> Result<()> {
        match src {
            IpAddr::V4(addr) => {
                self.set_src(addr);
                Ok(())
            }
            _ => Err(IpAddrMismatchError.into()),
        }
    }

    #[inline]
    fn dst(&self) -> IpAddr {
        IpAddr::V4(self.dst())
    }

    #[inline]
    fn set_dst(&mut self, dst: IpAddr) -> Result<()> {
        match dst {
            IpAddr::V4(addr) => {
                self.set_dst(addr);
                Ok(())
            }
            _ => Err(IpAddrMismatchError.into()),
        }
    }

    #[inline]
    fn pseudo_header(&self, packet_len: u16, protocol: ProtocolNumber) -> PseudoHeader {
        PseudoHeader::V4 {
            src: self.src(),
            dst: self.dst(),
            packet_len,
            protocol,
        }
    }
}
